use clap::Parser;
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "list",
    about = "List all rule files")]
#[display("list")]
pub struct List {}

impl Run for List {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        let rule_files = app_state.client.list(context::current()).await??;
        if rule_files.is_empty() {
            println!("No rule files found");
            return Ok(());
        }

        println!("\n    {:<5} {:<20}", "ID", "Name");

        for rule_file in rule_files {
            println!("    {:<5} {:<20}", rule_file.id, rule_file.name);
        }
        println!();

        Ok(())
    }
}