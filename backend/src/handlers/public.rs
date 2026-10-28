use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde_json::{json, Value};

use crate::{error::ApiResult, AppState};

pub async fn health() -> Json<Value> {
    Json(json!({
        "ok": true,
        "data": {
            "service": "driftnotes-api",
            "runtime": "rust",
            "storage": "sqlite+local"
        }
    }))
}

pub async fn list_posts(
    State(app): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.public_list(&query)?))
}

pub async fn get_post(
    State(app): State<AppState>,
    Path(slug): Path<String>,
) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.public_detail(&slug)?))
}

pub async fn list_tags(State(app): State<AppState>) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.tags()?))
}

pub async fn archive(State(app): State<AppState>) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.archive()?))
}

pub async fn settings(State(app): State<AppState>) -> ApiResult<Json<Value>> {
    Ok(Json(app.posts.settings()?))
}
