//! contains the init function for the reciever
use std::io::stdin;

use notify_rust::{Notification, Timeout, Urgency};
use tokio::io;

pub const APP_NAME: &str = env!("CARGO_PKG_NAME");

use crate::{
    Cipher, Path, collect_files,
    discovery::{
        connection::{Connection, Send, Serialize, Tcp, Udp},
        hostinfo::HostInfo,
        message::{Message, TransferDesision, TransferResponse},
        udp_logic::DiscoveryMessage,
    },
    encryption::{
        cipher::TransferError,
        key_agreement::{receiver::init_reciever_key_exchange, sender::keygen::Secret},
    },
    identity::{identity_definition::Identification, identity_exchange::key_exchange},
};

pub enum ConfirmMode {
    Terminal,
    Notification,
}

pub async fn init_receiver(confirmation_mode: &ConfirmMode) -> io::Result<Option<(Secret, Connection<Tcp>, usize)>> {
    let identification = Identification::new();
    let mut udp = Connection::<Udp>::new_listen().await?;

    let mut discoverer_msg = loop {
        match Message::receive::<DiscoveryMessage>(&mut udp).await {
            Ok(message) => break message,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => continue,
            Err(error) => return Err(error),
        }
    };
    let sender = udp
        .last_sender()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "couldnt get discovery sender"))?;
    discoverer_msg.set_ip(sender.ip());

    //connect to it
    let mut tcp_connection = Connection::<Tcp>::new_send_n_listen(discoverer_msg.socket(), udp).await?;

    //read_host
    let host = Message::receive::<HostInfo>(&mut tcp_connection).await?;

    //confirm if user wants to reciecve data
    let file_count = host.file_num();

    let was_accepted = match confirmation_mode {
        ConfirmMode::Notification => notify_connection(host, &mut tcp_connection).await?,
        ConfirmMode::Terminal => confirm_connection(host, &mut tcp_connection).await?,
    };

    if !was_accepted {
        return Ok(None);
    }
    //exchange keys
    let identity_context = key_exchange(&mut tcp_connection, identification).await?;

    //check if the recieved pubident hash matches the actual key exchange identity
    identity_context.verify_identifiy(discoverer_msg)?;

    let secret = init_reciever_key_exchange(&mut tcp_connection, &identity_context).await?;

    Ok(Some((secret, tcp_connection, file_count)))
}

async fn confirm_connection(hostinfo: HostInfo, connection: &mut Connection<Tcp>) -> io::Result<bool> {
    loop {
        let mut line = String::new();
        println!(
            "Do you wish to connect to {} to recive {} ammount of files (y/n)",
            hostinfo.name(),
            hostinfo.file_num()
        );

        stdin().read_line(&mut line)?;

        let line = line.trim().to_ascii_lowercase();

        match line.as_str() {
            "yes" | "y" => {
                println!("You accepted, continuing...");
                connection
                    .send(&TransferResponse::new(TransferDesision::Accepted)?.serialize())
                    .await?;
                return Ok(true);
            },
            "no" | "n" => {
                println!("You rejected, exiting program...");
                connection
                    .send(&TransferResponse::new(TransferDesision::Rejected)?.serialize())
                    .await?;

                return Ok(false);
            },
            _ => println!("Please enter y or n"),
        }
    }
}

async fn notify_connection(hostinfo: HostInfo, connection: &mut Connection<Tcp>) -> io::Result<bool> {
    let mut accepted = false;

    Notification::new()
        .summary(&format!("New incoming connection from {}", hostinfo.name()))
        .body(&format!(
            "Device {} wants to send {} files do you want to accept",
            hostinfo.name(),
            hostinfo.file_num()
        ))
        .appname(APP_NAME)
        .action("accepted", "Accept")
        .action("rejected", "Reject")
        .hint(notify_rust::Hint::Category("transfer".into()))
        .hint(notify_rust::Hint::Resident(true))
        .urgency(Urgency::Normal)
        .timeout(Timeout::Never)
        .show()
        .map_err(|_| io::Error::other("failed to show notification"))?
        .wait_for_action(|action| match action {
            "accepted" => accepted = true,
            "rejected" => accepted = false,
            _ => accepted = false,
        });

    match accepted {
        true => {
            connection
                .send(&TransferResponse::new(TransferDesision::Accepted)?.serialize())
                .await?;
            Ok(true)
        },
        false => {
            connection
                .send(&TransferResponse::new(TransferDesision::Rejected)?.serialize())
                .await?;

            Ok(false)
        },
    }
}

pub async fn receive_once(mode: ConfirmMode) -> Result<(), TransferError> {
    let Some((secret, mut tcp, file_count)) = init_receiver(&mode).await? else {
        return Ok(());
    };

    tokio::spawn(async move {
        let cipher = Cipher::try_from(secret)?;

        collect_files(file_count, Path::new("received_files"), &cipher, &mut tcp).await?;

        Ok::<(), TransferError>(())
    })
    .await??;

    Ok(())
}
