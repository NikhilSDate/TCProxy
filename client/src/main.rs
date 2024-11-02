use std::net::SocketAddr;

use clap::Parser;
use tarpc::{client, tokio_serde::formats::Json};

use shared::services::RuleSvcClient;
use crate::command::Run;

mod command;
pub mod io;
mod error;

#[derive(Parser)]
struct Flags {
    /// Remote RPC server address (ip:port)
    #[clap(short, long, default_value = "127.0.0.1:50050")]
    server_addr: SocketAddr,
}

pub struct AppState {
    client: RuleSvcClient,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let flags = Flags::parse();
    let mut transport = tarpc::serde_transport::tcp::connect(flags.server_addr, Json::default);
    transport.config_mut().max_frame_length(usize::MAX);

    let client = RuleSvcClient::new(client::Config::default(), transport.await?).spawn();

    let mut app_state = AppState { client };
    loop {
        let input = match io::readline(None) {
            Ok(input) => input,
            Err(err) => { println!("Input error: {:?}", err); continue; },
        };
        println!("Input: {:?}", input);
        let command = match command::Command::try_parse_from(format!("client {}", input).split(" ").collect::<Vec<&str>>()) {
            Ok(command) => command,
            Err(err) => { println!("Parse error: {}", err); continue; },
        };
        match command.run(&mut app_state).await {
            Ok(_) => {},
            Err(err) => { println!("Error executing command: {:?}", err); },
        };
    }
}
