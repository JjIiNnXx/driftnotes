use axum::extract::Multipart;
use chrono::{Datelike, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::{
    error::{ApiError, ApiResult},
    models::Asset,
    repositories::asset_repository::AssetRepository,
    storage::local::LocalStorage,
};

const MAX_FILE_SIZE: usize = 5 * 1024 * 1024;

#[derive(Clone)]
pub struct AssetService {
    assets: AssetRepository,
    storage: LocalStorage,
}

impl AssetService {
    pub fn new(assets: AssetRepository, storage: LocalStorage) -> Self {
        Self { assets, storage }
    }

    pub async fn upload(&self, mut multipart: Multipart) -> ApiResult<(Asset, bool)> {
        while let Some(field) = multipart
            .next_field()
            .await
            .map_err(|error| ApiError::bad_request(format!("Invalid multipart upload: {error}")))?
        {
            if field.name() != Some("file") {
                continue;
            }
            let filename = field
                .file_name()
                .map(str::to_owned)
                .unwrap_or_else(|| "upload".to_owned());
            let declared_type = field.content_type().map(str::to_owned);
            let bytes = field.bytes().await.map_err(|error| {
                if error.status() == axum::http::StatusCode::PAYLOAD_TOO_LARGE {
                    ApiError::too_large("Multipart request is too large")
                } else {
                    ApiError::bad_request(format!("Could not read uploaded file: {error}"))
                }
            })?;
            if bytes.len() > MAX_FILE_SIZE {
                return Err(ApiError::too_large("Uploaded file must not exceed 5 MiB"));
            }
            let (content_type, extension) = detect_image(&bytes).ok_or_else(|| {
                ApiError::bad_request("Only valid JPEG, PNG, GIF, and WebP images are accepted")
            })?;
            if declared_type
                .as_deref()
                .is_some_and(|value| value != content_type)
            {
                return Err(ApiError::bad_request(
                    "Declared Content-Type does not match the image bytes",
                ));
            }
            let hash = hex::encode(Sha256::digest(&bytes));
            if let Some(asset) = self.assets.find_by_hash(&hash)? {
                crate::logging::info(
                    "asset",
                    format!("reused hash={hash} key={} size={}", asset.key, asset.size),
                );
                return Ok((asset, true));
            }
            let now = Utc::now();
            let key = format!(
                "posts/{:04}/{:02}/{}.{}",
                now.year(),
                now.month(),
                hash,
                extension
            );
            let url = format!("/images/{key}");
            let asset = Asset {
                id: 0,
                key,
                filename: filename.chars().take(255).collect(),
                content_type: content_type.to_owned(),
                size: bytes.len() as i64,
                content_hash: hash,
                created_at: now.to_rfc3339_opts(SecondsFormat::Millis, true),
                url,
            };
            self.storage.save(&asset.key, &bytes).await?;
            if let Err(error) = self.assets.create(&asset) {
                let _ = self.storage.remove(&asset.key).await;
                return Err(error);
            }
            let id = self
                .assets
                .find_by_hash(&asset.content_hash)?
                .map(|asset| asset.id)
                .ok_or_else(|| ApiError::internal("Uploaded asset metadata was not saved"))?;
            let saved = self
                .assets
                .find_by_id(id)?
                .ok_or_else(|| ApiError::internal("Uploaded asset disappeared"))?;
            crate::logging::info(
                "asset",
                format!(
                    "uploaded hash={} key={} type={} size={}",
                    saved.content_hash, saved.key, saved.content_type, saved.size
                ),
            );
            return Ok((saved, false));
        }
        Err(ApiError::bad_request(
            "Multipart form must contain a file field named file",
        ))
    }

    pub fn list(&self) -> ApiResult<Value> {
        Ok(json!({ "data": { "items": self.assets.list()? } }))
    }

    pub fn delete(&self, id: i64) -> ApiResult<Value> {
        let asset = self
            .assets
            .find_by_id(id)?
            .ok_or_else(|| ApiError::not_found("Asset not found"))?;
        if self.assets.reference_count(&asset.url)? > 0 {
            return Err(ApiError::conflict(
                "ASSET_IN_USE",
                "Asset is referenced by a post and cannot be deleted",
            ));
        }
        Ok(json!({ "data": { "asset": asset } }))
    }

    pub async fn finish_delete(&self, id: i64) -> ApiResult<()> {
        let asset = self
            .assets
            .find_by_id(id)?
            .ok_or_else(|| ApiError::not_found("Asset not found"))?;
        if self.assets.reference_count(&asset.url)? > 0 {
            return Err(ApiError::conflict(
                "ASSET_IN_USE",
                "Asset is referenced by a post and cannot be deleted",
            ));
        }
        if !self.assets.delete(id)? {
            return Err(ApiError::not_found("Asset not found"));
        }
        self.storage.remove(&asset.key).await?;
        crate::logging::info(
            "asset",
            format!("deleted id={} key={}", asset.id, asset.key),
        );
        Ok(())
    }

    pub fn find_by_key(&self, key: &str) -> ApiResult<Asset> {
        self.assets
            .find_by_key(key)?
            .ok_or_else(|| ApiError::not_found("Asset not found"))
    }

    pub fn stats(&self) -> ApiResult<(i64, i64)> {
        self.assets.stats()
    }
}

fn detect_image(bytes: &[u8]) -> Option<(&'static str, &'static str)> {
    if bytes.starts_with(&[0xff, 0xd8, 0xff]) {
        Some(("image/jpeg", "jpg"))
    } else if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
        Some(("image/png", "png"))
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Some(("image/gif", "gif"))
    } else if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        Some(("image/webp", "webp"))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::detect_image;

    #[test]
    fn checks_magic_bytes_instead_of_extension() {
        assert_eq!(
            detect_image(&[0x89, b'P', b'N', b'G', 13, 10, 26, 10]),
            Some(("image/png", "png"))
        );
        assert_eq!(detect_image(b"not an image"), None);
    }
}
