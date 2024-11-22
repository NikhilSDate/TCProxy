use std::sync::{Arc, Mutex};
use rulelib::vm::Program;

#[derive(Debug, Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<rusqlite::Connection>>,
    pub program: Program
}
