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

#[tokio::main]
async fn main() -> Result<(), TransferError> {
    let commands = Cli::parse();

    match commands.command {
        StartSender(start_args) => {
            let headers = build_header(&start_args.directory)?;
            let counter = headers.len();

            //TODO who i wnat to send too instead of going into rety again
            let (secret, mut tcp) = loop {
                match sender_init(counter as u32).await {
                    Ok(connection) => break connection,
                    Err(crate::discovery::sender::init::SenderInitError::Rejected) => continue,
                    Err(crate::discovery::sender::init::SenderInitError::Io(error)) => return Err(error.into()),
                }
            };

            let cipher = Cipher::try_from(secret)?;

            send_files(&headers, &cipher, &mut tcp).await?;
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
