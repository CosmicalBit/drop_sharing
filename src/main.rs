use std::path::Path;

use clap::Parser;

use crate::{
    arguments::{
        Cli,
        Command::{StartReceiver, StartSender},
    },
    discovery::{
        receiver::init::{ConfirmMode, receive_once},
        sender::init::sender_init,
    },
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
mod wpa;

#[tokio::main]
async fn main() -> Result<(), TransferError> {
    let commands = Cli::parse();

    match commands.command {
        StartSender(start_args) => {
            let headers = build_header(&start_args.directory)?;
            let counter = headers.len();

            let (secret, mut tcp) = loop {
                match sender_init(counter as u32).await? {
                    Some(connection) => break connection,
                    None => continue,
                }
            };

            let cipher = Cipher::try_from(secret)?;

            send_files(&headers, &cipher, &mut tcp.tcp).await?;
            return Ok(());
        },

        StartReceiver(args) => {
            if args.daemon {
                loop {
                    receive_once(ConfirmMode::Notification).await?;
                }
            } else {
                receive_once(ConfirmMode::Terminal).await?;
            }
        },
    }

    Ok(())
}
