mod db;
mod error;
mod handlers;
mod logging;
mod models;
mod repositories;
mod router;
mod services;
mod storage;

use std::{
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use repositories::{
    asset_repository::AssetRepository, post_repository::PostRepository,
    settings_repository::SettingsRepository, SharedConnection,
};
use services::{asset_service::AssetService, post_service::PostService};
use storage::local::LocalStorage;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct AppState {
    pub posts: PostService,
    pub assets: AssetService,
    pub storage: LocalStorage,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // `cargo run` starts in `backend/`, so the default data directory is
    // backend/data. Override with BLOG_DATA_DIR when running the binary from
    // elsewhere.
    let data_dir = PathBuf::from(env::var("BLOG_DATA_DIR").unwrap_or_else(|_| "data".to_owned()));
    let storage_dir = data_dir.join("storage");
    let log_dir = env::var("BLOG_LOG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("backend crate lives inside the repository")
                .join("log")
        });
    let address: SocketAddr = env::var("BLOG_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8787".to_owned())
        .parse()?;

    logging::init(log_dir)?;
    logging::info(
        "app",
        format!("starting: api={address} data={}", data_dir.display()),
    );

    let connection = db::initialize(&data_dir, &storage_dir)?;
    let shared: SharedConnection = Arc::new(Mutex::new(connection));
    logging::info(
        "db",
        format!("sqlite ready: {}", data_dir.join("blog.db").display()),
    );

    let posts = PostService::new(
        PostRepository::new(shared.clone()),
        SettingsRepository::new(shared.clone()),
    );
    let storage = LocalStorage::new(storage_dir);
    let assets = AssetService::new(AssetRepository::new(shared), storage.clone());

    let state = AppState {
        posts,
        assets,
        storage,
    };
    let app = router::app(state);

    let listener = TcpListener::bind(address).await?;
    println!("qichi blog API listening on http://{address}");
    println!("data directory: {}", data_dir.display());
    logging::info("app", format!("listening on http://{address}"));
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    logging::info("app", "server stopped");

    Ok(())
}
