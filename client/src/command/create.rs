use std::fs;
use clap::{Parser, Subcommand};
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "create",
    about = "Create a new rule file")]
#[display("create")]

pub struct Create {
    #[clap(short, long, help = "The name of the rule file")]
    pub name: String,
    #[clap(short, long, help = "The path to the rule file")]
    pub path: String,
}

impl Run for Create {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        let content = fs::read_to_string(&self.path)?;
        let id = app_state.client.create(context::current(), self.name.clone(), content).await??;
        println!("Created rule file with id: {}", id);
        Ok(())
    }
}