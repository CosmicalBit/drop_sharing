use std::path::Path;

use clap::Parser;

use crate::{
    arguments::{
        Cli,
        Command::{StartReceiver, StartSender},
    },
    discovery::{receiver::init::init_receiver, sender::init::sender_init},
    encryption::cipher::{Cipher, TransferError},
    file_collection::{
        send_chunked::{collect_files, send_files},
        tree_walking::build_header,
    },
};

mod arguments;
mod discovery;
mod encryption;
mod file_collection;
mod identity;

#[tokio::main]
async fn main() -> Result<(), TransferError> {
    let commands = Cli::parse();

    match commands.command {
        StartSender(start_args) => {
            let headers = build_header(&start_args.directory)?;
            let counter = headers.len();

            let (secret, mut tcp) = sender_init(counter as u32).await?;

            let cipher = Cipher::try_from(secret)?;

            send_files(&headers, &cipher, &mut tcp).await?;
        },

        StartReceiver => {
            let (secret, mut tcp, file_count) = init_receiver().await?;

            let cipher = Cipher::try_from(secret)?;

            collect_files(file_count, Path::new("received_files"), &cipher, &mut tcp).await?;
        },
    }

    Ok(())
}
