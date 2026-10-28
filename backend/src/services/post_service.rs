use std::collections::{HashMap, HashSet};

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};

use crate::{
    error::{ApiError, ApiResult},
    models::{
        ArchiveMonth, ArchiveYear, CreatePostRequest, Post, PostSummary, PostWrite,
        UpdatePostRequest,
    },
    repositories::{post_repository::PostRepository, settings_repository::SettingsRepository},
};

use super::markdown;

#[derive(Clone)]
pub struct PostService {
    posts: PostRepository,
    settings: SettingsRepository,
}

impl PostService {
    pub fn new(posts: PostRepository, settings: SettingsRepository) -> Self {
        Self { posts, settings }
    }

    pub fn public_list(&self, query: &HashMap<String, String>) -> ApiResult<Value> {
        let (page, page_size) = pagination(query)?;
        let tag = query.get("tag").map(String::as_str);
        let search = normalized_search(query.get("q"));
        let (posts, total) =
            self.posts
                .list(true, None, tag, search.as_deref(), page, page_size)?;
        Ok(paginated(posts, total, page, page_size))
    }

    pub fn admin_list(&self, query: &HashMap<String, String>) -> ApiResult<Value> {
        let (page, page_size) = pagination(query)?;
        let status = query.get("status").map(String::as_str);
        if let Some(status) = status {
            validate_status(status)?;
        }
        let search = normalized_search(query.get("q"));
        let (posts, total) =
            self.posts
                .list(false, status, None, search.as_deref(), page, page_size)?;
        Ok(paginated(posts, total, page, page_size))
    }

    pub fn public_detail(&self, slug: &str) -> ApiResult<Value> {
        let post = self
            .posts
            .find_published_by_slug(slug)?
            .ok_or_else(|| ApiError::not_found("Published post not found"))?;
        let published_at = post
            .published_at
            .clone()
            .ok_or_else(|| ApiError::internal("Published post has no published_at"))?;
        let (previous, next) = self.posts.previous_next(post.id, &published_at)?;
        Ok(json!({
            "data": {
                "post": post,
                "previous": previous.map(PostSummary::from),
                "next": next.map(PostSummary::from),
            }
        }))
    }

    pub fn admin_detail(&self, id: i64) -> ApiResult<Value> {
        let post = self.find(id)?;
        Ok(json!({ "data": { "post": post } }))
    }

    pub fn tags(&self) -> ApiResult<Value> {
        Ok(json!({ "data": self.posts.tags_with_counts()? }))
    }

    pub fn archive(&self) -> ApiResult<Value> {
        let mut years: Vec<ArchiveYear> = Vec::new();
        for post in self.posts.published_archive()? {
            let Some(published_at) = post.published_at.as_deref() else {
                continue;
            };
            if published_at.len() < 7 {
                continue;
            }
            let Ok(year) = published_at[..4].parse::<i32>() else {
                continue;
            };
            let Ok(month) = published_at[5..7].parse::<u32>() else {
                continue;
            };
            if years.last().map(|group| group.year) != Some(year) {
                years.push(ArchiveYear {
                    year,
                    months: Vec::new(),
                });
            }
            let year_group = years.last_mut().expect("year was just inserted");
            if year_group.months.last().map(|group| group.month) != Some(month) {
                year_group.months.push(ArchiveMonth {
                    month,
                    posts: Vec::new(),
                });
            }
            year_group
                .months
                .last_mut()
                .expect("month was just inserted")
                .posts
                .push(PostSummary::from(post));
        }
        Ok(json!({ "data": years }))
    }

    pub fn settings(&self) -> ApiResult<Value> {
        Ok(json!({ "data": { "settings": self.settings.all()? } }))
    }

    pub fn create(&self, request: CreatePostRequest) -> ApiResult<Post> {
        if self.posts.slug_exists(&request.slug, None)? {
            return Err(ApiError::conflict(
                "SLUG_EXISTS",
                "Post slug already exists",
            ));
        }
        let status = request.status.as_deref().unwrap_or("draft");
        validate_status(status)?;
        let title = required_text(&request.title, "title", 200)?;
        let slug = validate_slug(&request.slug)?;
        let content_markdown =
            required_text(&request.content_markdown, "content_markdown", 1024 * 1024)?;
        let content_html = markdown::render(&content_markdown)?;
        let excerpt = normalize_excerpt(request.excerpt, &content_markdown);
        let now = now();
        let published_at = normalize_date(request.published_at)?
            .or_else(|| (status == "published").then(|| now.clone()));
        let tag_names = normalize_tags(request.tags);
        validate_tag_names(&tag_names)?;
        let tag_ids = self.resolve_tags(&tag_names)?;
        let post = PostWrite {
            title,
            slug,
            excerpt,
            content_markdown,
            content_html,
            status: status.to_owned(),
            published_at,
            created_at: now.clone(),
            updated_at: now,
            cover_image: normalize_cover(request.cover_image)?,
        };
        let id = self.posts.create(&post, &tag_ids)?;
        let post = self.find(id)?;
        crate::logging::info(
            "post",
            format!(
                "created id={} slug={} status={} tags={}",
                post.id,
                post.slug,
                post.status,
                post.tags.len()
            ),
        );
        Ok(post)
    }

    pub fn update(&self, id: i64, request: UpdatePostRequest) -> ApiResult<Post> {
        if request.version < 1 {
            return Err(ApiError::bad_request("version must be a positive integer"));
        }
        let existing = self.find(id)?;
        if existing.version != request.version {
            return Err(version_conflict());
        }
        if existing.status == "published"
            && request
                .slug
                .as_deref()
                .is_some_and(|slug| slug != existing.slug)
        {
            return Err(ApiError::conflict(
                "SLUG_IMMUTABLE",
                "The slug of a published post cannot be changed",
            ));
        }
        let slug = match request.slug {
            Some(slug) => validate_slug(&slug)?,
            None => existing.slug.clone(),
        };
        if slug != existing.slug && self.posts.slug_exists(&slug, Some(id))? {
            return Err(ApiError::conflict(
                "SLUG_EXISTS",
                "Post slug already exists",
            ));
        }
        let title = match request.title {
            Some(title) => required_text(&title, "title", 200)?,
            None => existing.title.clone(),
        };
        let content_markdown = match request.content_markdown {
            Some(content) => required_text(&content, "content_markdown", 1024 * 1024)?,
            None => existing.content_markdown.clone(),
        };
        let content_html = markdown::render(&content_markdown)?;
        let status = match request.status {
            Some(status) => {
                validate_status(&status)?;
                status
            }
            None => existing.status.clone(),
        };
        let published_at = normalize_date(request.published_at)?
            .or_else(|| existing.published_at.clone())
            .or_else(|| (status == "published").then(now));
        let tag_ids = match request.tags {
            Some(tags) => {
                let tags = normalize_tags(Some(tags));
                validate_tag_names(&tags)?;
                Some(self.resolve_tags(&tags)?)
            }
            None => None,
        };
        let post = PostWrite {
            title,
            slug,
            excerpt: normalize_excerpt(request.excerpt, &content_markdown),
            content_markdown,
            content_html,
            status,
            published_at,
            created_at: existing.created_at,
            updated_at: now(),
            cover_image: match request.cover_image {
                Some(cover) => normalize_cover(Some(cover))?,
                None => existing.cover_image,
            },
        };
        if !self
            .posts
            .update(id, request.version, &post, tag_ids.as_deref())?
        {
            return Err(version_conflict());
        }
        let saved = self.find(id)?;
        crate::logging::info(
            "post",
            format!(
                "updated id={} slug={} status={} version={}",
                saved.id, saved.slug, saved.status, saved.version
            ),
        );
        Ok(saved)
    }

    pub fn delete(&self, id: i64) -> ApiResult<Post> {
        if !self.posts.archive(id, &now())? {
            return Err(ApiError::not_found("Post not found"));
        }
        let post = self.find(id)?;
        crate::logging::info(
            "post",
            format!("archived id={} slug={}", post.id, post.slug),
        );
        Ok(post)
    }

    pub fn status_counts(&self) -> ApiResult<(i64, i64, i64, i64)> {
        self.posts.status_counts()
    }

    fn find(&self, id: i64) -> ApiResult<Post> {
        self.posts
            .find_by_id(id)?
            .ok_or_else(|| ApiError::not_found("Post not found"))
    }

    fn resolve_tags(&self, tags: &[String]) -> ApiResult<Vec<i64>> {
        self.posts.resolve_tag_ids(tags)
    }
}

fn pagination(query: &HashMap<String, String>) -> ApiResult<(i64, i64)> {
    let page = positive_integer(query.get("page"), 1, i64::MAX, "page")?;
    let page_size = positive_integer(
        query.get("page_size").or_else(|| query.get("pageSize")),
        10,
        100,
        "page_size",
    )?;
    Ok((page, page_size))
}

fn positive_integer(
    value: Option<&String>,
    default: i64,
    maximum: i64,
    name: &str,
) -> ApiResult<i64> {
    let Some(value) = value.filter(|value| !value.is_empty()) else {
        return Ok(default);
    };
    let parsed = value
        .parse::<i64>()
        .ok()
        .filter(|number| *number >= 1 && *number <= maximum)
        .ok_or_else(|| ApiError::bad_request(format!("{name} must be between 1 and {maximum}")))?;
    Ok(parsed)
}

fn paginated(posts: Vec<Post>, total: i64, page: i64, page_size: i64) -> Value {
    let items = posts.into_iter().map(PostSummary::from).collect::<Vec<_>>();
    json!({
        "data": {
            "items": items,
            "pagination": {
                "page": page,
                "page_size": page_size,
                "total": total,
                "total_pages": if total == 0 { 0 } else { (total + page_size - 1) / page_size },
            }
        }
    })
}

fn normalized_search(value: Option<&String>) -> Option<String> {
    value.map(|value| value.trim().chars().take(200).collect())
}

fn validate_status(status: &str) -> ApiResult<()> {
    if matches!(status, "published" | "draft" | "archived") {
        Ok(())
    } else {
        Err(ApiError::bad_request(
            "status must be one of published, draft, archived",
        ))
    }
}

fn required_text(value: &str, field: &str, maximum: usize) -> ApiResult<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(format!(
            "{field} is required and must be a non-empty string"
        )));
    }
    if value.len() > maximum {
        return Err(ApiError::bad_request(format!(
            "{field} must not exceed {maximum} bytes"
        )));
    }
    Ok(trimmed.to_owned())
}

fn validate_slug(slug: &str) -> ApiResult<String> {
    let slug = required_text(slug, "slug", 160)?;
    let valid = !slug.is_empty()
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--")
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-');
    if valid {
        Ok(slug)
    } else {
        Err(ApiError::invalid_slug())
    }
}

fn normalize_date(value: Option<String>) -> ApiResult<Option<String>> {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|date| {
                    date.with_timezone(&Utc)
                        .to_rfc3339_opts(SecondsFormat::Millis, true)
                })
                .map_err(|_| {
                    ApiError::bad_request("published_at must be a valid RFC 3339 date-time")
                })
        })
        .transpose()
}

fn normalize_excerpt(value: Option<String>, markdown_source: &str) -> Option<String> {
    value
        .and_then(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.chars().take(300).collect())
        })
        .or_else(|| Some(markdown::excerpt(markdown_source, 180)))
}

fn normalize_cover(value: Option<String>) -> ApiResult<Option<String>> {
    let Some(value) = value else { return Ok(None) };
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    if value.len() > 500 || !(value.starts_with("/covers/") || value.starts_with("/images/")) {
        return Err(ApiError::bad_request(
            "cover_image must be a local /covers/ or /images/ URL",
        ));
    }
    Ok(Some(value.to_owned()))
}

fn normalize_tags(tags: Option<Vec<String>>) -> Vec<String> {
    let mut seen = HashSet::new();
    tags.unwrap_or_default()
        .into_iter()
        .map(|tag| tag.trim().to_owned())
        .filter(|tag| !tag.is_empty())
        .filter(|tag| seen.insert(tag.to_lowercase()))
        .collect()
}

fn validate_tag_names(tags: &[String]) -> ApiResult<()> {
    if tags.len() > 20
        || tags
            .iter()
            .any(|tag| tag.trim().is_empty() || tag.len() > 60)
    {
        return Err(ApiError::bad_request(
            "tags must contain at most 20 non-empty names of at most 60 bytes",
        ));
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn version_conflict() -> ApiError {
    ApiError::conflict(
        "VERSION_CONFLICT",
        "Post was modified by another editor; reload and retry",
    )
}
