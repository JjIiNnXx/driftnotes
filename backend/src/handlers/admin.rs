use std::collections::HashMap;

use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::{
    error::{ApiError, ApiResult},
    models::{CreatePostRequest, RenderRequest, UpdatePostRequest},
    services::markdown,
    AppState,
};

pub async fn dashboard_stats(State(app): State<AppState>) -> ApiResult<Json<Value>> {
    let (total, published, draft, archived) = app.posts.status_counts()?;
    let (asset_count, asset_size_bytes) = app.assets.stats()?;
    Ok(Json(json!({
        "data": {
            "total_posts": total,
            "published_posts": published,
            "draft_posts": draft,
            "archived_posts": archived,
            "asset_count": asset_count,
            "asset_size_bytes": asset_size_bytes,
        }
    })))
}

pub async fn list_posts(
    State(app): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.admin_list(&query)?))
}

pub async fn get_post(State(app): State<AppState>, Path(id): Path<i64>) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.admin_detail(id)?))
}

pub async fn create_post(
    State(app): State<AppState>,
    Json(request): Json<CreatePostRequest>,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let post = app.posts.create(request)?;
    Ok((
        StatusCode::CREATED,
        Json(json!({ "data": { "post": post } })),
    ))
}

pub async fn update_post(
    State(app): State<AppState>,
    Path(id): Path<i64>,
    Json(request): Json<UpdatePostRequest>,
) -> ApiResult<Json<Value>> {
    let post = app.posts.update(id, request)?;
    Ok(Json(json!({ "data": { "post": post } })))
}

pub async fn archive_post(
    State(app): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Value>> {
    let post = app.posts.delete(id)?;
    Ok(Json(json!({ "data": { "post": post } })))
}

pub async fn render_preview(Json(request): Json<RenderRequest>) -> ApiResult<Json<Value>> {
    let html = markdown::render(&request.markdown)?;
    Ok(Json(json!({ "data": { "html": html } })))
}

pub async fn upload_asset(
    State(app): State<AppState>,
    multipart: Multipart,
) -> ApiResult<(StatusCode, Json<Value>)> {
    let (asset, deduplicated) = app.assets.upload(multipart).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "data": {
                "asset": asset,
                "deduplicated": deduplicated
            }
        })),
    ))
}

pub async fn list_assets(State(app): State<AppState>) -> ApiResult<Json<Value>> {
    Ok(Json(app.assets.list()?))
}

pub async fn delete_asset(
    State(app): State<AppState>,
    Path(id): Path<i64>,
) -> ApiResult<Json<Value>> {
    app.assets.finish_delete(id).await?;
    Ok(Json(json!({ "data": { "deleted": true } })))
}

pub async fn not_found() -> ApiError {
    ApiError::not_found("Route not found")
}
