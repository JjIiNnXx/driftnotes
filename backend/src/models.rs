use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub slug: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TagCount {
    #[serde(flatten)]
    pub tag: Tag,
    pub post_count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Post {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub content_markdown: String,
    pub content_html: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
    pub cover_image: Option<String>,
    pub tags: Vec<Tag>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PostSummary {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub version: i64,
    pub cover_image: Option<String>,
    pub tags: Vec<Tag>,
}

impl From<Post> for PostSummary {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            slug: post.slug,
            excerpt: post.excerpt,
            status: post.status,
            published_at: post.published_at,
            created_at: post.created_at,
            updated_at: post.updated_at,
            version: post.version,
            cover_image: post.cover_image,
            tags: post.tags,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PostWrite {
    pub title: String,
    pub slug: String,
    pub excerpt: Option<String>,
    pub content_markdown: String,
    pub content_html: String,
    pub status: String,
    pub published_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub cover_image: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: String,
    pub content_markdown: String,
    pub excerpt: Option<String>,
    pub status: Option<String>,
    pub published_at: Option<String>,
    pub cover_image: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub version: i64,
    pub title: Option<String>,
    pub slug: Option<String>,
    pub content_markdown: Option<String>,
    pub excerpt: Option<String>,
    pub status: Option<String>,
    pub published_at: Option<String>,
    pub cover_image: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct RenderRequest {
    pub markdown: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Asset {
    pub id: i64,
    pub key: String,
    pub filename: String,
    pub content_type: String,
    pub size: i64,
    pub content_hash: String,
    pub created_at: String,
    pub url: String,
}

#[derive(Debug, Serialize)]
pub struct ArchiveYear {
    pub year: i32,
    pub months: Vec<ArchiveMonth>,
}

#[derive(Debug, Serialize)]
pub struct ArchiveMonth {
    pub month: u32,
    pub posts: Vec<PostSummary>,
}
