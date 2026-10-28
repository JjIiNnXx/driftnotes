use std::collections::BTreeMap;

use crate::error::{ApiError, ApiResult};

use super::SharedConnection;

#[derive(Clone)]
pub struct SettingsRepository {
    db: SharedConnection,
}

impl SettingsRepository {
    pub fn new(db: SharedConnection) -> Self {
        Self { db }
    }

    pub fn all(&self) -> ApiResult<BTreeMap<String, serde_json::Value>> {
        let connection = self.db.lock().map_err(ApiError::internal)?;
        let mut statement = connection.prepare("SELECT key, value FROM settings ORDER BY key")?;
        let rows = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        let mut settings = BTreeMap::new();
        for row in rows {
            let (key, value) = row?;
            settings.insert(
                key,
                serde_json::from_str(&value).unwrap_or(serde_json::Value::String(value)),
            );
        }
        Ok(settings)
    }
}
