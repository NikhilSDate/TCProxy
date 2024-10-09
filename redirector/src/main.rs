use std::net::Ipv4Addr;
use std::str::FromStr;

use clap::Parser;
use futures::{future, StreamExt};
use tarpc::server::incoming::Incoming;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;

use crate::redirector::redirect;
use crate::rpc::init_rpc;

mod redirector;
mod rpc;

#[derive(Parser, Debug)]
#[clap(name = "Reverse TCP Proxy", version="0.1.0", author="Ronan Boyarski, Nikil Date, Ethan Zhang, Somrishi Bannerjee")]
struct Args {
    // Redirection
    #[clap(short = 'b', long, help = "Local port to bind to")]
    bind_port: u16,
    #[clap(short = 'l', long, default_value = "0.0.0.0", help = "Local IP to bind to")]
    bind_ip: Ipv4Addr,
    #[clap(short, long, help = "Destination port to forward to")]
    dest_port: u16,
    #[clap(short = 'r', long, default_value = "127.0.0.1", help = "Destination IP to forward to")]
    dest_ip: Ipv4Addr,
    // Interactive Settings (for non-daemon mode)
    #[clap(short = 's', long, help = "Log to stdout instead of a file")]
    stdout: bool,
    // Logging configuration
    #[clap(long, default_value = "info", help = "Maximum log level to display")]
    log_level: Level,
    #[clap(long, default_value = "log", help = "Directory to store logs")]
    log_dir: String,
    #[clap(long, default_value = "connections.log", help = "File to store logs")]
    log_file: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(args.log_level)
        .with_writer(move || -> Box<dyn std::io::Write> {
            match args.stdout {
                true => Box::new(std::io::stdout()),
                false => Box::new(RollingFileAppender::new(Rotation::DAILY, args.log_dir.clone(), args.log_file.clone()))
            }
        })
        .init();

    // Start redirector
    tokio::spawn(async move { redirect(args.bind_ip, args.bind_port, args.dest_ip, args.dest_port).await } );

    // Start RPC server
    tokio::spawn(async move { init_rpc().await });

    // Wait for both to finish
    future::pending::<()>().await;
}

#[cfg(test)]
mod tests {
    // Included temporarily to test GitHub Actions workflow
    #[tokio::test]
    async fn sanity_check() {
        assert_eq!(1+1, 2);
    }
}