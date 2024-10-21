/// Frontend that can interact with RPC
/// Code mostly stolen from documentation
use clap::Parser;
use std::{net::SocketAddr, time::Duration};
use tarpc::{client, context, tokio_serde::formats::Json};
use tokio::time::sleep;

#[derive(Parser)]
struct Flags {
    /// Remote RPC server address (ip:port)
    #[clap(long)]
    server_addr: SocketAddr,
    /// Sets the name to say hello to.
    #[clap(long)]
    name: String,
}

#[tarpc::service]
pub trait World {
    /// Returns a greeting for name.
    async fn hello(name: String) -> String;
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let flags = Flags::parse();
    let mut transport = tarpc::serde_transport::tcp::connect(flags.server_addr, Json::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = WorldClient::new(client::Config::default(), transport.await?).spawn();

    let hello = async move {
        tokio::select! {
            hello1 = client.hello(context::current(), format!("{}1", flags.name)) => { hello1 }
        }
    }
        .await;

    match hello {
        Ok(hello) => println!("{hello:?}"),
        Err(e) => println!("{:?}", anyhow::Error::from(e)),
    }

    // Let the background span processor finish.
    sleep(Duration::from_micros(1)).await;

    Ok(())
}
