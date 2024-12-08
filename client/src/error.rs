use derive_more::From;
use tarpc::client::RpcError;

#[derive(Debug, From)]
pub enum Error {
    #[from]
    Shared(shared::error::Error),
    #[from]
    Anyhow(anyhow::Error),
    #[from]
    Io(std::io::Error),
    #[from]
    Signal(reedline::Signal),
    #[from]
    RpcError(RpcError),
}

pub type Result<T> = core::result::Result<T, Error>;
