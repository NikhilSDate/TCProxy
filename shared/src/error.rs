use tarpc::serde::Serialize;
use serde::Deserialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Error {
    Anyhow(String)
}

pub type Result<T> = core::result::Result<T, Error>;