//! contains the init function for the reciever
use std::io::stdin;

use tokio::io;

use crate::{
    discovery::{
        connection::{Connection, Send, Serialize, Tcp, Udp},
        hostinfo::HostInfo,
        message::{Message, TransferDesision, TransferResponse},
        udp_logic::DiscoveryMessage,
    },
    encryption::key_agreement::{receiver::init_reciever_key_exchange, sender::keygen::Secret},
    identity::{identity_definition::Identification, identity_exchange::key_exchange},
};

pub async fn init_receiver() -> io::Result<(Secret, Connection<Tcp>, usize)> {
    let identification = Identification::new();
    let mut udp = Connection::<Udp>::new_listen().await?;

    let discoverer_msg = loop {
        match Message::receive::<DiscoveryMessage>(&mut udp).await {
            Ok(message) => break message,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => continue,
            Err(error) => return Err(error),
        }
    };

    //connect to it
    let mut tcp_connection = Connection::<Tcp>::new_send_n_listen(discoverer_msg.socket(), udp).await?;

    //read_host
    let host = Message::receive::<HostInfo>(&mut tcp_connection).await?;

    //confirm if user wants to reciecve data
    let file_count = host.file_num();
    confirm_connection(host, &mut tcp_connection).await?;

    //exchange keys
    let identity_context = key_exchange(&mut tcp_connection, identification).await?;

    //check if the recieved pubident hash matches the actual key exchange identity
    identity_context.verify_identifiy(discoverer_msg)?;

    let secret = init_reciever_key_exchange(&mut tcp_connection, &identity_context).await?;

    Ok((secret, tcp_connection, file_count))
}

async fn confirm_connection(hostinfo: HostInfo, connection: &mut Connection<Tcp>) -> io::Result<()> {
    loop {
        let mut line = String::new();
        println!(
            "do you wish to connect to {} to recive {} ammount of files (y/n)",
            hostinfo.name(),
            hostinfo.file_num()
        );

        stdin().read_line(&mut line)?;

        let line = line.trim().to_ascii_lowercase();

        match line.as_str() {
            "yes" | "y" => {
                println!("you accepted, continuing...");
                connection
                    .send(&TransferResponse::new(TransferDesision::Accepted)?.serialize())
                    .await?;
                return Ok(());
            },
            "no" | "n" => {
                println!("you rejected, exiting program...");
                connection
                    .send(&TransferResponse::new(TransferDesision::Rejected)?.serialize())
                    .await?;

                return Err(io::Error::new(std::io::ErrorKind::InvalidData, "user decided to abort"));
            },
            _ => println!("please enter y or n"),
        }
    }
}
