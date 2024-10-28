use clap::Parser;
use derive_more::Display;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "exit",
    about = "Exits immediately")]
#[display("exit")]

pub struct Exit {}

impl Run for Exit {
    async fn run(&self, _app_state: &AppState) -> Result<()> {
        println!("Goodbye!");
        std::process::exit(0);
    }
}