use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<rusqlite::Connection>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFile {
    id: i32,
    name: String,
    content: String,
}
