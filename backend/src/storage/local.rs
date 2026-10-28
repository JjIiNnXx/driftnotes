use std::path::{Component, Path, PathBuf};

use tokio::{fs, io::AsyncWriteExt};

use crate::error::{ApiError, ApiResult};

#[derive(Clone)]
pub struct LocalStorage {
    root: PathBuf,
}

impl LocalStorage {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    fn path_for(&self, key: &str) -> ApiResult<PathBuf> {
        let path = Path::new(key);
        if path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, Component::Normal(_)))
        {
            return Err(ApiError::bad_request("Invalid asset key"));
        }
        Ok(self.root.join(path))
    }

    pub async fn save(&self, key: &str, bytes: &[u8]) -> ApiResult<()> {
        let path = self.path_for(key)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await?;
        file.write_all(bytes).await?;
        Ok(())
    }

    pub async fn read(&self, key: &str) -> ApiResult<Vec<u8>> {
        let path = self.path_for(key)?;
        fs::read(path).await.map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                ApiError::not_found("Asset file not found")
            } else {
                ApiError::internal(error)
            }
        })
    }

    pub async fn remove(&self, key: &str) -> ApiResult<()> {
        let path = self.path_for(key)?;
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(ApiError::internal(error)),
        }
    }
}
