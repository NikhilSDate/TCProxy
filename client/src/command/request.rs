use std::fs;
use clap::Parser;
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "request",
    about = "Request a rule file by id")]
#[display("request")]
pub struct Request {
    #[clap(short, long, help = "The id of the rule file")]
    pub id: i64,
}

impl Run for Request {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        let rule_file = app_state.client.request(context::current(), self.id.clone()).await??;
        println!("Got rule file (id {}):\n{}", &self.id, rule_file.content);
        Ok(())
    }
}