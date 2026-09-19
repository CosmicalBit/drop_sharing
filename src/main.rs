use clap::Parser;
use tokio::io;

use crate::{
    arguments::{
        Cli,
        Command::{StartReceiver, StartSender},
    },
    discovery::{receiver::init::init_receiver, sender::init::sender_init},
    file_collection::tree_walking::{FileCount, walk},
};

mod arguments;
mod discovery;
mod encryption;
mod file_collection;
mod identity;

#[tokio::main]
async fn main() -> io::Result<()> {
    let commands = Cli::parse();

    match commands.command {
        StartSender(start_args) => {
            let mut counter = FileCount::default();

            walk(&start_args.directory, &mut counter)?;

            sender_init(counter.count() as u32).await?;
        },
        StartReceiver => {
            init_receiver().await?;
        },
    }

    Ok(())
}
