use std::path::PathBuf;

use crate::ui::BackendCommunication;
use clap::{Args, Parser, Subcommand};
use eframe::wgpu::Backend;
#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    GUI,
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
