use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    StartSender(StartArgs),
    StartReciever,
}

#[derive(Parser)]
pub struct StartArgs {
    pub directory: PathBuf,
    pub name: Option<String>,
}
