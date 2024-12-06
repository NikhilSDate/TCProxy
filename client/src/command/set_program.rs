use std::fs;
use clap::Parser;
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "set_program",
    about = "Set a program to run")]
#[display("set_program")]
pub struct SetProgram {
    #[clap(help = "The ID of the program to set")]
    pub id: i64,
}

impl Run for SetProgram {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        app_state.client.set_program(context::current(), self.id).await??;
        println!("Set program with id: {}", self.id);
        Ok(())
    }
}