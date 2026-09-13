use clap::Parser;

use crate::arguments::{Cli, Command::Start};

mod arguments;
mod discover;
mod start_handler;

fn main() {
    let commands = Cli::parse();

    match commands.command {
        Start(start_args) => todo!(),

        _ => unreachable!(),
    }
}
