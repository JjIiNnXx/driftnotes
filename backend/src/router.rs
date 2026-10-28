use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, post},
    Router,
};
use tower_http::cors::CorsLayer;

use crate::{
    handlers::{admin, images, public},
    logging, AppState,
};

pub fn app(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(public::health))
        .route("/api/posts", get(public::list_posts))
        .route("/api/posts/{slug}", get(public::get_post))
        .route("/api/tags", get(public::list_tags))
        .route("/api/archive", get(public::archive))
        .route("/api/settings", get(public::settings))
        .route("/api/admin/stats", get(admin::dashboard_stats))
        .route(
            "/api/admin/posts",
            get(admin::list_posts).post(admin::create_post),
        )
        .route(
            "/api/admin/posts/{id}",
            get(admin::get_post)
                .put(admin::update_post)
                .delete(admin::archive_post),
        )
        .route("/api/admin/render", post(admin::render_preview))
        .route(
            "/api/admin/assets",
            get(admin::list_assets).post(admin::upload_asset),
        )
        .route("/api/admin/assets/{id}", delete(admin::delete_asset))
        .route("/images/{*key}", get(images::serve_image))
        .fallback(admin::not_found)
        .layer(DefaultBodyLimit::max(6 * 1024 * 1024))
        .layer(CorsLayer::permissive())
        .layer(middleware::from_fn(logging::request_log))
        .with_state(state)
}
