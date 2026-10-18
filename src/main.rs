use crate::arguments::Command::{self, GUI};
use crate::file_collection::tree_walking::FileHeader;
use crate::settings::Configs;
use crate::ui::{AppData, BackendCommunication, Communication, CommunicationMode};
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
use clap::Parser;
use std::io;
use std::path::PathBuf;
use tokio::sync::watch;

mod arguments;
mod discovery;
mod encryption;
mod file_collection;
mod identity;
mod settings;
mod ui;
mod wpa;

#[tokio::main]
async fn main() -> Result<(), TransferError> {
    let commands = Cli::parse();

    let command = commands.command.unwrap_or(Command::GUI);

    let config = Configs::new()?;

    match command {
        StartSender(start_args) => {
            let headers = build_header(&start_args.directory)?;
            let counter = headers.len();

            let (secret, mut tcp) = loop {
                match sender_init(counter as u32, None).await? {
                    Some(connection) => break connection,
                    None => continue,
                }
            };

            let cipher = Cipher::try_from(secret)?;

            send_files(&headers, &cipher, &mut tcp.tcp, None).await?;
            return Ok(());
        },

        StartReceiver(args) => {
            if args.daemon {
                loop {
                    receive_once(ConfirmMode::Notification, None, &config).await?;
                }
            } else {
                receive_once(ConfirmMode::Terminal, None, &config).await?;
            }
        },
        GUI => {
            // Initialize all channels
            let (devices_tx, devices_rx) = watch::channel(None::<Vec<String>>);
            let (chosen_receiver_tx, chosen_receiver_rx) = watch::channel(None::<String>);
            let (progress_tx, progress_rx) = watch::channel(None::<f32>);
            let (receive_answer_tx, receive_answer_rx) = watch::channel(None::<bool>);
            let (incoming_tx, incoming_rx) = watch::channel(None::<crate::discovery::hostinfo::HostInfo>);
            let (selected_files_tx, selected_files_rx) = watch::channel(None::<Vec<PathBuf>>);
            let (mode_tx, mode_rx) = watch::channel(None::<CommunicationMode>);

            let communication = Communication::new(
                devices_rx,
                chosen_receiver_tx,
                progress_rx,
                receive_answer_tx,
                incoming_rx,
                selected_files_tx,
                mode_tx,
            );
            let mut backend_communication = BackendCommunication::new(
                devices_tx,
                chosen_receiver_rx,
                progress_tx,
                receive_answer_rx,
                incoming_tx,
                selected_files_rx,
                mode_rx,
            );
            let app = Box::new(AppData::new(communication));

            tokio::spawn(async move {
                let mode = crate::ui::receive_value(&mut backend_communication.mode).await?;

                match mode {
                    CommunicationMode::Sender => {
                        //start sender
                        let files = crate::ui::receive_value(&mut backend_communication.selected_files_to_send).await?;

                        let headers: Vec<FileHeader> = files
                            .iter()
                            .map(|file| build_header(file))
                            .collect::<io::Result<Vec<Vec<FileHeader>>>>()?
                            .into_iter()
                            .flatten()
                            .collect();
                        let counter = headers.len();

                        let (secret, mut tcp) = loop {
                            match sender_init(counter as u32, Some(&mut backend_communication)).await? {
                                Some(connection) => break connection,
                                None => continue,
                            }
                        };

                        let cipher = Cipher::try_from(secret)?;

                        send_files(&headers, &cipher, &mut tcp.tcp, Some(&backend_communication)).await?;
                    },

                    CommunicationMode::Reciever => {
                        //start reciever

                        receive_once(ConfirmMode::Notification, Some(&mut backend_communication), &config).await?;
                    },
                }

                Ok::<(), TransferError>(())
            });

            eframe::run_native("Drop Sharing", eframe::NativeOptions::default(), Box::new(move |_cc| Ok(app)))?;
        },
    }

    Ok(())
}
