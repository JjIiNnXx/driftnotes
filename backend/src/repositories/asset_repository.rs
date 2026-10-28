use rusqlite::{params, Connection, OptionalExtension, Row};

use crate::{
    error::{ApiError, ApiResult},
    models::Asset,
};

use super::SharedConnection;

#[derive(Clone)]
pub struct AssetRepository {
    db: SharedConnection,
}

impl AssetRepository {
    pub fn new(db: SharedConnection) -> Self {
        Self { db }
    }

    fn connection(&self) -> ApiResult<std::sync::MutexGuard<'_, Connection>> {
        self.db.lock().map_err(ApiError::internal)
    }

    pub fn list(&self) -> ApiResult<Vec<Asset>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "SELECT id, key, filename, content_type, size, content_hash, created_at \
             FROM assets ORDER BY created_at DESC, id DESC",
        )?;
        let rows = statement.query_map([], row_to_asset)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn find_by_id(&self, id: i64) -> ApiResult<Option<Asset>> {
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT id, key, filename, content_type, size, content_hash, created_at \
                 FROM assets WHERE id = ?1",
                params![id],
                row_to_asset,
            )
            .optional()?)
    }

    pub fn find_by_hash(&self, hash: &str) -> ApiResult<Option<Asset>> {
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT id, key, filename, content_type, size, content_hash, created_at \
                 FROM assets WHERE content_hash = ?1",
                params![hash],
                row_to_asset,
            )
            .optional()?)
    }

    pub fn find_by_key(&self, key: &str) -> ApiResult<Option<Asset>> {
        let connection = self.connection()?;
        Ok(connection
            .query_row(
                "SELECT id, key, filename, content_type, size, content_hash, created_at \
                 FROM assets WHERE key = ?1",
                params![key],
                row_to_asset,
            )
            .optional()?)
    }

    pub fn create(&self, asset: &Asset) -> ApiResult<i64> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO assets (key, filename, content_type, size, content_hash, created_at) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                asset.key,
                asset.filename,
                asset.content_type,
                asset.size,
                asset.content_hash,
                asset.created_at,
            ],
        )?;
        Ok(connection.last_insert_rowid())
    }

    pub fn reference_count(&self, url: &str) -> ApiResult<i64> {
        let connection = self.connection()?;
        Ok(connection.query_row(
            "SELECT COUNT(*) FROM posts WHERE cover_image = ?1 \
             OR instr(content_markdown, ?1) > 0 OR instr(content_html, ?1) > 0",
            params![url],
            |row| row.get(0),
        )?)
    }

    pub fn delete(&self, id: i64) -> ApiResult<bool> {
        let connection = self.connection()?;
        Ok(connection.execute("DELETE FROM assets WHERE id = ?1", params![id])? > 0)
    }

    pub fn stats(&self) -> ApiResult<(i64, i64)> {
        let connection = self.connection()?;
        Ok(connection.query_row(
            "SELECT COUNT(*), COALESCE(SUM(size), 0) FROM assets",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?)
    }
}

fn row_to_asset(row: &Row<'_>) -> rusqlite::Result<Asset> {
    let key: String = row.get(1)?;
    Ok(Asset {
        id: row.get(0)?,
        url: format!("/images/{key}"),
        key,
        filename: row.get(2)?,
        content_type: row.get(3)?,
        size: row.get(4)?,
        content_hash: row.get(5)?,
        created_at: row.get(6)?,
    })
}
