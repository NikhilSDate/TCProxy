use std::net::Ipv4Addr;
use tokio::io::copy_bidirectional;
use tokio::net::TcpListener;
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

        tokio::spawn(async move {
            match copy_bidirectional(&mut inbound, &mut outbound).await {
                Ok(_) => {},
                Err(e) => {
                    error!("Error forwarding connection: {}", e);
                }
            };
        });
    }
}