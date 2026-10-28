pub mod asset_repository;
pub mod post_repository;
pub mod settings_repository;

use std::sync::{Arc, Mutex};

use rusqlite::Connection;

pub type SharedConnection = Arc<Mutex<Connection>>;
