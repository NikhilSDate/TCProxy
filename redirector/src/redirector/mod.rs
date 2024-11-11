use futures::{future, FutureExt, StreamExt, TryFutureExt};
use std::cell::RefCell;
use std::io;
use std::net::Ipv4Addr;
use std::task::{ready, Poll};
use tokio::io::{copy_bidirectional, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::macros::support::poll_fn;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::bytes::Bytes;
use tokio_util::io::{ReaderStream, StreamReader};
use tracing::{error, event, info, Level};

pub async fn redirect(bind_ip: Ipv4Addr, bind_port: u16, dest_ip: Ipv4Addr, dest_port: u16) {
    let listener = TcpListener::bind(format!("{}:{}", bind_ip, bind_port))
        .await
        .unwrap(); // We should panic here as a failure this early is unrecoverable

    event!(
        Level::INFO,
        "Forwarding from {}:{} to {}:{}",
        bind_ip,
        bind_port,
        dest_ip,
        dest_port
    );

    while let Ok((mut inbound, _)) = listener.accept().await {
        println!("Connection opened");
        let mut outbound =
            match tokio::net::TcpStream::connect(format!("{}:{}", dest_ip, dest_port)).await {
                Ok(s) => {
                    info!(
                        "Received connection from {}",
                        inbound
                            .peer_addr()
                            .expect("Failed to parse inbound address")
                    );
                    s
                }
                Err(e) => {
                    error!("Error connecting to destination: {}", e);
                    continue;
                }
            };

        let mut reader_stream = ReaderStream::new(inbound);

        while let Some(result) = reader_stream.next().await {
            match result {
                Ok(bytes) => {
                    outbound.write_all(&bytes).await;  
                }
                Err(e) => {
                    break;
                }
            }
        }
    }
}
