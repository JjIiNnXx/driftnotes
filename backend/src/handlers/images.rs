use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::Response,
};

use crate::{error::ApiError, AppState};

pub async fn serve_image(
    State(app): State<AppState>,
    Path(key): Path<String>,
) -> Result<Response, ApiError> {
    let key = key.trim_start_matches('/');
    let bytes = app.storage.read(key).await?;
    let mime = mime_guess::from_path(key)
        .first_or_octet_stream()
        .to_string();

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .header(header::CACHE_CONTROL, "public, max-age=31536000, immutable")
        .header(header::X_CONTENT_TYPE_OPTIONS, "nosniff")
        .body(Body::from(bytes))
        .map_err(ApiError::internal)
}
