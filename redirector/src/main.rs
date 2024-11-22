use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use clap::Parser;
use futures::future;
use rusqlite::Connection;
use tracing::Level;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber;
use rulelib::vm::Instruction::{DROP, ITE, REDIRECT, SEQ};
use rulelib::vm::{Object, PACKET_SOURCE_IP, Program};
use crate::model::AppState;
use crate::redirector::redirect;
use crate::rpc::init_rpc;

mod redirector;
mod rpc;
mod sql;
pub mod model;

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
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Fake IP checker program (pre-compiled)
    // If IP != localhost DROP else REDIRECT to DEST_IP:PORT
    let insns = vec![
        SEQ(0, 0, PACKET_SOURCE_IP),
        ITE(0, 2, 3),
        REDIRECT(1, 2),
        DROP
    ];

    let mut data = HashMap::new();
    data.insert(0, Object::IP(Ipv4Addr::new(127, 0, 0, 1)));
    data.insert(1, Object::IP(args.dest_ip));
    data.insert(2, Object::Port(args.dest_port));
    let program = Program {
        instructions: insns,
        data,
    };

    let app_state = AppState{
        conn: Arc::new(Mutex::new(Connection::open_in_memory()?)),
        program
    };

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
    let binding = app_state.clone();
    tokio::spawn(async move { redirect(args.bind_ip, args.bind_port, args.dest_ip, args.dest_port, binding).await } );

    // Start RPC server
    tokio::spawn(async move { init_rpc(app_state).await });

    // Wait for both to finish
    future::pending::<()>().await;

    Ok(())
}