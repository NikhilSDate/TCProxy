use std::io;
use std::net::Ipv4Addr;
use std::task::{Poll, ready};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, copy_bidirectional};
use tokio::macros::support::poll_fn;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, event, info, Level};

pub async fn redirect(bind_ip: Ipv4Addr, bind_port: u16, dest_ip: Ipv4Addr, dest_port: u16) {
    let listener = TcpListener::bind(format!("{}:{}", bind_ip, bind_port))
        .await.unwrap(); // We should panic here as a failure this early is unrecoverable

    event!(Level::INFO,"Forwarding from {}:{} to {}:{}", bind_ip, bind_port, dest_ip, dest_port);

    while let Ok((mut inbound, _)) = listener.accept().await {
        let mut outbound = match tokio::net::TcpStream::connect(format!("{}:{}", dest_ip, dest_port))
            .await {
            Ok(s) => {
                info!("Received connection from {}", inbound.peer_addr()
                    .expect("Failed to parse inbound address"));
                s
            },
            Err(e) => {
                error!("Error connecting to destination: {}", e);
                continue;
            }
        };

        let (mut out_in, mut out_out) = outbound.into_split();
        let (mut in_in, mut in_out) = inbound.into_split();

        tokio::spawn(async move {
            loop {
                let mut buffer = [0; 1024 * 8];
                out_in.read(&mut buffer).await.unwrap();
                // println!("Received: {:?}", buffer);
                in_out.write(&mut buffer).await.unwrap();
            }
        });


        tokio::spawn(async move {
            loop {
                let mut buffer = [0; 1024 * 8];
                in_in.read(&mut buffer).await.unwrap();
                // println!("Received: {:?}", buffer);
                out_out.write_all(&buffer).await.unwrap();
            }
        });

    }
}