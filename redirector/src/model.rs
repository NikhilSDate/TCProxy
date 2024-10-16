use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<rusqlite::Connection>>
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFile {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) content: String,
}
