use crate::error::Result;
use crate::model::RuleFile;

#[tarpc::service]
pub trait RuleSvc {
    async fn create(name: String, content: String) -> Result<i64>;
    async fn list() -> Result<Vec<RuleFile>>;
    async fn request(id: i64) -> Result<RuleFile>;
    async fn update(id: i64, content: String) -> Result<()>;
    async fn delete(id: i64) -> Result<()>;
    async fn set_program(id: i64) -> Result<()>;
}