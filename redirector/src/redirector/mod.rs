use futures::{future, FutureExt, StreamExt, TryFutureExt};
use std::cell::RefCell;
use std::io;
use std::net::Ipv4Addr;
use std::process::Output;
use std::task::{ready, Poll};
use tokio::io::{copy_bidirectional, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::macros::support::poll_fn;
use tokio::net::{TcpListener, TcpStream};
use core::net::SocketAddr;
use std::sync::Arc;
use std::str::FromStr;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::bytes::Bytes;
use tokio_util::io::{ReaderStream, StreamReader};
use tracing::{error, event, info, Level};
use rulelib::vm::{Action, Packet, VM};
use crate::model::AppState;

fn convert_to_packet(local_addr: SocketAddr, peer_addr: SocketAddr, content: Bytes) -> Packet {
    Packet {
        // Disgusting type nonsense
        source: (Ipv4Addr::from_str(&local_addr.ip().to_string()).unwrap(), local_addr.port()),
        dest: (Ipv4Addr::from_str(&peer_addr.ip().to_string()).unwrap(), peer_addr.port()),
        content: Arc::new(Vec::from(content)),
    }
}

fn filter(packet: Packet, app_state: &AppState) -> Action {
    let mut vm = VM::new();
    // ok to unwrap here: if the unwrap fails something has gone very wrong
    let program = app_state.program.lock().unwrap();
    let result = vm.run_program(&*program, &packet);
    if result.is_err() {
        error!("Error running program: {:?}", result.err().unwrap());
        return Action::DROP;
    }
    result.unwrap()
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

        // Unwrapping because if we can't get this, something has gone terribly wrong anyway
        let local_addr = inbound.local_addr().unwrap();
        let peer_addr = inbound.peer_addr().unwrap();

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
                        let packet = convert_to_packet(local_addr, peer_addr, bytes);

                        // Get another handle to packet content so we can modify it in place
                        let content = packet.content.clone();

                        match filter(packet, &binding) {
                            Action::REDIRECT(destination, port) => {
                                if let Some(e) = otx.write_all(&**content).await.err() {
                                    error!("Error writing to outbound stream: {:?}", e);
                                    break;
                                }
                            }
                            Action::DROP => { continue; }
                            Action::REJECT => { continue; }
                            Action::REWRITE(destination, port) => {
                                if let Some(e) = otx.write_all(&**content).await.err() {
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
