use clap::Parser;
use clap::Subcommand;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Cli {
    #[command(subcommand)]
   pub command: Command,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    Start(StartArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct StartArgs {
    pub directory: PathBuf,
    pub name: Option<String>,
    
}

