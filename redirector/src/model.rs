use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<rusqlite::Connection>>
}

