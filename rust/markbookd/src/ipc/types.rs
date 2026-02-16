use std::path::PathBuf;

use rusqlite::Connection;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct Request {
    pub id: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

pub struct AppState {
    pub workspace: Option<PathBuf>,
    pub db: Option<Connection>,
}
