use clap::Parser;
use derive_more::Display;
use tarpc::context;
use crate::command::Run;
use crate::error::Result;
use crate::AppState;

#[derive(Parser, Debug, Default, Display)]
#[clap(
    name = "delete",
    about = "Delete a rule file by id")]
#[display("delete")]

pub struct Delete {
    #[clap(short, long, help = "The id of the rule file")]
    pub id: i64,
}

impl Run for Delete {
    async fn run(&self, app_state: &AppState) -> Result<()> {
        app_state.client.delete(context::current(), self.id.clone()).await??;
        println!("Deleted rule file (id {})", &self.id);
        Ok(())
    }
}