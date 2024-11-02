use std::fs;
use clap::Parser;
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "update",
    about = "Update an existing rule file")]
#[display("update")]
pub struct Update {
    #[clap(short, long, help = "The id of the rule file")]
    pub id: i64,
    #[clap(short, long, help = "The path to the new rule file")]
    pub path: String,
}

impl Run for Update {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        let content = fs::read_to_string(&self.path)?;
        app_state.client.update(context::current(), self.id.clone(), content).await??;
        println!("Updated rule file (id {}) to match {}", self.id, self.path);
        Ok(())
    }
}