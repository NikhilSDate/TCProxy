use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleFile {
    pub id: i64,
    pub name: String,
    pub content: String,
}