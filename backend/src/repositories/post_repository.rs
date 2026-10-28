use rusqlite::{params, params_from_iter, types::Value, Connection, OptionalExtension, Row};
use sha2::{Digest, Sha256};

use crate::{
    error::{ApiError, ApiResult},
    models::{Post, PostWrite, Tag, TagCount},
};

use super::SharedConnection;

const POST_SELECT: &str = r#"
    SELECT id, title, slug, excerpt, content_markdown, content_html, status,
           published_at, created_at, updated_at, version, cover_image
    FROM posts p
"#;

#[derive(Clone)]
pub struct PostRepository {
    db: SharedConnection,
}

impl PostRepository {
    pub fn new(db: SharedConnection) -> Self {
        Self { db }
    }

    fn connection(&self) -> ApiResult<std::sync::MutexGuard<'_, Connection>> {
        self.db.lock().map_err(ApiError::internal)
    }

    pub fn list(
        &self,
        public_only: bool,
        status: Option<&str>,
        tag: Option<&str>,
        search: Option<&str>,
        page: i64,
        page_size: i64,
    ) -> ApiResult<(Vec<Post>, i64)> {
        let mut conditions = Vec::new();
        let mut values = Vec::new();
        if public_only {
            conditions.push("p.status = 'published'".to_owned());
        } else if let Some(status) = status.filter(|value| !value.is_empty()) {
            conditions.push("p.status = ?".to_owned());
            values.push(Value::from(status.to_owned()));
        }
        if let Some(tag) = tag.filter(|value| !value.is_empty()) {
            conditions.push(
                "EXISTS (SELECT 1 FROM post_tags pt JOIN tags t ON t.id = pt.tag_id \
                 WHERE pt.post_id = p.id AND (t.slug = ? OR t.name = ?))"
                    .to_owned(),
            );
            values.push(Value::from(tag.to_owned()));
            values.push(Value::from(tag.to_owned()));
        }
        if let Some(search) = search.filter(|value| !value.is_empty()) {
            conditions.push(
                "(p.title LIKE ? OR p.excerpt LIKE ? OR p.content_markdown LIKE ?)".to_owned(),
            );
            let pattern = format!("%{search}%");
            values.push(Value::from(pattern.clone()));
            values.push(Value::from(pattern.clone()));
            values.push(Value::from(pattern));
        }
        let condition = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };
        let order = if public_only {
            "ORDER BY p.published_at DESC, p.id DESC"
        } else {
            "ORDER BY p.updated_at DESC, p.id DESC"
        };
        let connection = self.connection()?;
        let total: i64 = connection.query_row(
            &format!("SELECT COUNT(*) FROM posts p {condition}"),
            params_from_iter(values.iter()),
            |row| row.get(0),
        )?;
        let mut page_values = values;
        page_values.push(Value::from(page_size));
        page_values.push(Value::from((page - 1) * page_size));
        let posts = query_posts(
            &connection,
            &format!("{POST_SELECT} {condition} {order} LIMIT ? OFFSET ?"),
            &page_values,
        )?;
        Ok((posts, total))
    }

    pub fn slug_exists(&self, slug: &str, exclude_id: Option<i64>) -> ApiResult<bool> {
        let connection = self.connection()?;
        let count: i64 = match exclude_id {
            Some(id) => connection.query_row(
                "SELECT COUNT(*) FROM posts WHERE slug = ?1 AND id != ?2",
                params![slug, id],
                |row| row.get(0),
            )?,
            None => connection.query_row(
                "SELECT COUNT(*) FROM posts WHERE slug = ?1",
                params![slug],
                |row| row.get(0),
            )?,
        };
        Ok(count > 0)
    }

    pub fn find_by_id(&self, id: i64) -> ApiResult<Option<Post>> {
        let connection = self.connection()?;
        find_one(
            &connection,
            &format!("{POST_SELECT} WHERE p.id = ?1"),
            &[Value::from(id)],
        )
    }

    pub fn find_published_by_slug(&self, slug: &str) -> ApiResult<Option<Post>> {
        let connection = self.connection()?;
        find_one(
            &connection,
            &format!("{POST_SELECT} WHERE p.slug = ?1 AND p.status = 'published'"),
            &[Value::from(slug.to_owned())],
        )
    }

    pub fn published_archive(&self) -> ApiResult<Vec<Post>> {
        let connection = self.connection()?;
        query_posts(
            &connection,
            &format!(
                "{POST_SELECT} WHERE p.status = 'published' \
                 ORDER BY p.published_at DESC, p.id DESC"
            ),
            &[],
        )
    }

    pub fn previous_next(
        &self,
        id: i64,
        published_at: &str,
    ) -> ApiResult<(Option<Post>, Option<Post>)> {
        let connection = self.connection()?;
        let previous = find_one(
            &connection,
            &format!(
                "{POST_SELECT} WHERE p.status = 'published' AND \
                 (p.published_at < ?1 OR (p.published_at = ?1 AND p.id < ?2)) \
                 ORDER BY p.published_at DESC, p.id DESC LIMIT 1"
            ),
            &[Value::from(published_at.to_owned()), Value::from(id)],
        )?;
        let next = find_one(
            &connection,
            &format!(
                "{POST_SELECT} WHERE p.status = 'published' AND \
                 (p.published_at > ?1 OR (p.published_at = ?1 AND p.id > ?2)) \
                 ORDER BY p.published_at ASC, p.id ASC LIMIT 1"
            ),
            &[Value::from(published_at.to_owned()), Value::from(id)],
        )?;
        Ok((previous, next))
    }

    pub fn tags_with_counts(&self) -> ApiResult<Vec<TagCount>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            r#"
            SELECT t.id, t.name, t.slug, COUNT(p.id) AS post_count
            FROM tags t
            LEFT JOIN post_tags pt ON pt.tag_id = t.id
            LEFT JOIN posts p ON p.id = pt.post_id AND p.status = 'published'
            GROUP BY t.id, t.name, t.slug
            ORDER BY t.name COLLATE NOCASE
            "#,
        )?;
        let rows = statement.query_map([], |row| {
            Ok(TagCount {
                tag: Tag {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    slug: row.get(2)?,
                },
                post_count: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Resolves tag names or slugs to ids. New tags are created on demand so
    /// the editor can introduce a topic without a separate management step.
    pub fn resolve_tag_ids(&self, tags: &[String]) -> ApiResult<Vec<i64>> {
        let connection = self.connection()?;
        let mut ids = Vec::with_capacity(tags.len());
        for tag in tags {
            if let Some(id) = connection
                .query_row(
                    "SELECT id FROM tags WHERE slug = ?1 OR name = ?1",
                    params![tag],
                    |row| row.get(0),
                )
                .optional()?
            {
                ids.push(id);
                continue;
            }

            let base = tag_slug(tag);
            let mut slug = base.clone();
            let mut suffix = 1;
            while connection
                .query_row(
                    "SELECT id FROM tags WHERE slug = ?1",
                    params![slug],
                    |row| row.get::<_, i64>(0),
                )
                .optional()?
                .is_some()
            {
                slug = format!("{base}-{suffix}");
                suffix += 1;
            }
            connection.execute(
                "INSERT INTO tags (name, slug) VALUES (?1, ?2)",
                params![tag, slug],
            )?;
            ids.push(connection.last_insert_rowid());
        }
        Ok(ids)
    }

    pub fn create(&self, post: &PostWrite, tag_ids: &[i64]) -> ApiResult<i64> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            r#"
            INSERT INTO posts (
                title, slug, excerpt, content_markdown, content_html, status,
                published_at, created_at, updated_at, version, cover_image
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 1, ?10)
            "#,
            params![
                post.title,
                post.slug,
                post.excerpt,
                post.content_markdown,
                post.content_html,
                post.status,
                post.published_at,
                post.created_at,
                post.updated_at,
                post.cover_image,
            ],
        )?;
        let id = transaction.last_insert_rowid();
        for tag_id in tag_ids {
            transaction.execute(
                "INSERT INTO post_tags (post_id, tag_id) VALUES (?1, ?2)",
                params![id, tag_id],
            )?;
        }
        transaction.commit()?;
        Ok(id)
    }

    pub fn update(
        &self,
        id: i64,
        expected_version: i64,
        post: &PostWrite,
        tag_ids: Option<&[i64]>,
    ) -> ApiResult<bool> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let affected = transaction.execute(
            r#"
            UPDATE posts SET
                title = ?1, slug = ?2, excerpt = ?3, content_markdown = ?4,
                content_html = ?5, status = ?6, published_at = ?7,
                created_at = ?8, updated_at = ?9, version = version + 1,
                cover_image = ?10
            WHERE id = ?11 AND version = ?12
            "#,
            params![
                post.title,
                post.slug,
                post.excerpt,
                post.content_markdown,
                post.content_html,
                post.status,
                post.published_at,
                post.created_at,
                post.updated_at,
                post.cover_image,
                id,
                expected_version,
            ],
        )?;
        if affected == 0 {
            return Ok(false);
        }
        if let Some(tag_ids) = tag_ids {
            transaction.execute("DELETE FROM post_tags WHERE post_id = ?1", params![id])?;
            for tag_id in tag_ids {
                transaction.execute(
                    "INSERT INTO post_tags (post_id, tag_id) VALUES (?1, ?2)",
                    params![id, tag_id],
                )?;
            }
        }
        transaction.commit()?;
        Ok(true)
    }

    pub fn archive(&self, id: i64, timestamp: &str) -> ApiResult<bool> {
        let connection = self.connection()?;
        let affected = connection.execute(
            "UPDATE posts SET status = 'archived', updated_at = ?1, version = version + 1 \
             WHERE id = ?2",
            params![timestamp, id],
        )?;
        Ok(affected > 0)
    }

    pub fn status_counts(&self) -> ApiResult<(i64, i64, i64, i64)> {
        let connection = self.connection()?;
        connection
            .query_row(
                r#"
                SELECT COUNT(*),
                       SUM(CASE WHEN status = 'published' THEN 1 ELSE 0 END),
                       SUM(CASE WHEN status = 'draft' THEN 1 ELSE 0 END),
                       SUM(CASE WHEN status = 'archived' THEN 1 ELSE 0 END)
                FROM posts
                "#,
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .map_err(ApiError::from)
    }
}

fn row_to_post(row: &Row<'_>) -> rusqlite::Result<Post> {
    Ok(Post {
        id: row.get(0)?,
        title: row.get(1)?,
        slug: row.get(2)?,
        excerpt: row.get(3)?,
        content_markdown: row.get(4)?,
        content_html: row.get(5)?,
        status: row.get(6)?,
        published_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
        version: row.get(10)?,
        cover_image: row.get(11)?,
        tags: Vec::new(),
    })
}

fn tags_for(connection: &Connection, post_id: i64) -> ApiResult<Vec<Tag>> {
    let mut statement = connection.prepare(
        r#"
        SELECT t.id, t.name, t.slug FROM tags t
        JOIN post_tags pt ON pt.tag_id = t.id
        WHERE pt.post_id = ?1 ORDER BY t.name
        "#,
    )?;
    let rows = statement.query_map(params![post_id], |row| {
        Ok(Tag {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<Vec<_>, _>>()?)
}

fn query_posts(connection: &Connection, sql: &str, values: &[Value]) -> ApiResult<Vec<Post>> {
    let mut posts = {
        let mut statement = connection.prepare(sql)?;
        let rows = statement.query_map(params_from_iter(values.iter()), row_to_post)?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    for post in &mut posts {
        post.tags = tags_for(connection, post.id)?;
    }
    Ok(posts)
}

/// Turns a human tag name into a stable URL segment. Names that contain no
/// ASCII letters or digits (for example, pure Chinese tags) get a short hash
/// suffix so the slug stays portable in URLs.
fn tag_slug(name: &str) -> String {
    let mut slug = String::new();
    let mut pending_dash = false;
    for character in name.chars() {
        if character.is_ascii_alphanumeric() {
            if pending_dash && !slug.is_empty() {
                slug.push('-');
            }
            pending_dash = false;
            slug.push(character.to_ascii_lowercase());
        } else {
            pending_dash = true;
        }
    }
    let slug: String = slug.chars().take(48).collect();
    let slug = slug.trim_end_matches('-');
    if slug.is_empty() {
        let digest = Sha256::digest(name.as_bytes());
        format!("tag-{}", &hex::encode(digest)[..10])
    } else {
        slug.to_owned()
    }
}

fn find_one(connection: &Connection, sql: &str, values: &[Value]) -> ApiResult<Option<Post>> {
    let post = connection
        .query_row(sql, params_from_iter(values.iter()), row_to_post)
        .optional()?;
    match post {
        Some(mut post) => {
            post.tags = tags_for(connection, post.id)?;
            Ok(Some(post))
        }
        None => Ok(None),
    }
}
