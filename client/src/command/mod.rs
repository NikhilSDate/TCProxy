mod exit;
mod create;
mod request;
mod update;
mod delete;

use clap::Parser;
use derive_more::Display;
use crate::AppState;
use crate::error::Result;
use strum::IntoEnumIterator;
use strum_macros::{EnumIter, EnumString};

#[derive(Parser, EnumIter, EnumString, Display)]
pub enum Command {
    Exit(exit::Exit),
    Create(create::Create),
    Request(request::Request),
    Update(update::Update),
    Delete(delete::Delete),
}

pub trait Run {
    async fn run(&self, app_state: &AppState) -> Result<()>;
}

impl Run for Command {
    // TODO find a way to macro this away because this sucks
    async fn run(&self, app_state: &AppState) -> Result<()> {
        match self {
            Command::Exit(exit) => exit.run(app_state).await,
            Command::Create(create) => create.run(app_state).await,
            Command::Request(request) => request.run(app_state).await,
            Command::Update(update) => update.run(app_state).await,
            Command::Delete(delete) => delete.run(app_state).await,
        }
    }
}

/// Lists all commands in Vector form for syntax highlighting
impl Command {
    pub fn all_commands() -> Vec<String> {
        let mut commands = vec!["help".to_string() /* included by clap */];
        for command in Command::iter() {
            commands.push(format!("{}", command));
        }
        commands
    }
}