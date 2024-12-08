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
        println!("Got rule files {:?}", rule_files);
        Ok(())
    }
}