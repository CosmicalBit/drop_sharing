use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    StartSender(StartArgs),
    StartReceiver(ReceiverArgs),
}

#[derive(Args)]
pub struct StartArgs {
    pub directory: PathBuf,
    pub name: Option<String>,
}

#[derive(Args)]
pub struct ReceiverArgs {
    #[arg(long, default_value_t = false)]
    pub daemon: bool,
}
