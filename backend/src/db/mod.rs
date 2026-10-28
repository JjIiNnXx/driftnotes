pub mod seed;

use std::{fs, path::Path};

use rusqlite::Connection;

use crate::error::ApiResult;

pub fn initialize(data_dir: &Path, storage_dir: &Path) -> ApiResult<Connection> {
    fs::create_dir_all(data_dir)?;
    fs::create_dir_all(storage_dir)?;
    let mut connection = Connection::open(data_dir.join("blog.db"))?;
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;
        PRAGMA journal_mode = WAL;
        PRAGMA busy_timeout = 5000;

        CREATE TABLE IF NOT EXISTS posts (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            slug TEXT NOT NULL UNIQUE,
            excerpt TEXT,
            content_markdown TEXT NOT NULL,
            content_html TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('published', 'draft', 'archived')),
            published_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            version INTEGER NOT NULL DEFAULT 1,
            cover_image TEXT
        );

        CREATE TABLE IF NOT EXISTS tags (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL UNIQUE,
            slug TEXT NOT NULL UNIQUE
        );

        CREATE TABLE IF NOT EXISTS post_tags (
            post_id INTEGER NOT NULL REFERENCES posts(id) ON DELETE CASCADE,
            tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            PRIMARY KEY (post_id, tag_id)
        );

        CREATE TABLE IF NOT EXISTS assets (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT NOT NULL UNIQUE,
            filename TEXT NOT NULL,
            content_type TEXT NOT NULL,
            size INTEGER NOT NULL,
            content_hash TEXT NOT NULL UNIQUE,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_posts_status_published
            ON posts(status, published_at DESC);
        CREATE INDEX IF NOT EXISTS idx_post_tags_tag ON post_tags(tag_id, post_id);
        "#,
    )?;
    seed::seed(&mut connection)?;
    Ok(connection)
}
