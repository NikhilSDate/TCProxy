use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr};
use futures::{future, StreamExt};
use tarpc::{context, server, server::Channel};
use tarpc::server::incoming::Incoming;
use tarpc::tokio_serde::formats::Json;
use tracing::{event, Level};

/// Constant value for where the RPC server binds to
const RPC_BIND: (Ipv4Addr, u16) = (Ipv4Addr::LOCALHOST, 50050);

#[tarpc::service]
trait World {
    async fn hello(name: String) -> String;
}

#[derive(Clone)]
struct HelloServer(SocketAddr);

impl World for HelloServer {
    async fn hello(self, _: context::Context, name: String) -> String {
        format!("Hello, {name}! You are connected from {}", self.0)
    }
}

async fn spawn(fut: impl Future<Output = ()> + Send + 'static) {
    tokio::spawn(fut);
}

/// Start the RPC server
pub async fn init_rpc() {
    let mut listener = tarpc::serde_transport::tcp::listen(&RPC_BIND, Json::default).await.expect("Failed to bind RPC listener");

    event!(Level::INFO, "RPC listening on {}:{}", RPC_BIND.0, RPC_BIND.1);

    listener.config_mut().max_frame_length(usize::MAX);
    listener
        .filter_map(|r| future::ready(r.ok()))
        .map(server::BaseChannel::with_defaults)
        .max_channels_per_key(1, |t| t.transport().peer_addr().unwrap().ip())
        .map(|channel| {
            let server = HelloServer(channel.transport().peer_addr().unwrap());
            channel.execute(server.serve()).for_each(spawn)
        })
        .buffer_unordered(10)
        .for_each(|_| async {})
        .await;
}
