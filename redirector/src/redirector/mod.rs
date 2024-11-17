use futures::{future, FutureExt, StreamExt, TryFutureExt};
use std::cell::RefCell;
use std::io;
use std::net::Ipv4Addr;
use std::process::Output;
use std::task::{ready, Poll};
use tokio::io::{copy_bidirectional, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::macros::support::poll_fn;
use tokio::net::{TcpListener, TcpStream};
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::bytes::Bytes;
use tokio_util::io::{ReaderStream, StreamReader};
use tracing::{error, event, info, Level};
use crate::model::AppState;

enum Action {
    FORWARD(Bytes),
    DROP,
    REJECT,
    REWRITE(Bytes),
}

fn filter(bytes: Bytes, app_state: &AppState) -> Action {
    // TODO - connect up to rule file and do real redirection
    let needle = Bytes::from_static(b"feroxbuster");
    if needle.len() > bytes.len() {
        return Action::FORWARD(bytes);
    }


    match bytes.windows(needle.len()).any(|window| window == needle) {
        true => {
            Action::REJECT
        }
        false => {
            Action::FORWARD(bytes)
        }
    }
}

pub async fn redirect(bind_ip: Ipv4Addr, bind_port: u16, dest_ip: Ipv4Addr, dest_port: u16, app_state: AppState) {
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

        
        let (orx, mut otx) = outbound.into_split();
        let (irx, mut itx) = inbound.into_split();

        let mut inbound_reader_stream = ReaderStream::new(irx);
        let mut outbound_reader_stream = ReaderStream::new(orx);

        let binding = app_state.clone();
        tokio::spawn(async move {
            while let Some(result) = inbound_reader_stream.next().await {
                match result {
                    Ok(bytes) => {
                        match filter(bytes, &binding) {
                            Action::FORWARD(bytes) => {
                                if let Some(e) = otx.write_all(&bytes).await.err() {
                                    error!("Error writing to outbound stream: {:?}", e);
                                    break;
                                }
                            }
                            Action::DROP => { continue; }
                            Action::REJECT => { continue; }
                            Action::REWRITE(new_bytes) => {
                                if let Some(e) = otx.write_all(&new_bytes).await.err() {
                                    error!("Error writing to outbound stream: {:?}", e);
                                    break;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        break;
                    }
                }
            }
        });

        tokio::spawn(async move {
            while let Some(result) = outbound_reader_stream.next().await {
                match result {
                    Ok(bytes) => {
                        itx.write_all(&bytes).await;  
                    }
                    Err(e) => {
                        break;
                    }
                }
            }
        });

    }
}
