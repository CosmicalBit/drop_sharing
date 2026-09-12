use clap::Parser;

use crate::arguments::{Cli, Command::Start};

mod arguments;
mod start_handler;
mod discover;


fn main() {
    let commands = Cli::parse();

    match commands.command {
        Start(start_args) => todo!(),

        _ => unreachable!(),
    }


    
}


