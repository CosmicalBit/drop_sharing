//! contains the init function for the reciever
use std::io::{Error, stdin};

use tokio::io;

use crate::{
    discovery::{
        connection::{Connection, Send, Serialize, Tcp, Udp},
        hostinfo::HostInfo,
        message::{Message, TransferDesision, TransferResponse},
        udp_logic::DiscoveryMessage,
    },
    encryption::key_agreement::receiver::init_reciever_key_exchange,
    identity::{identity_definition::Identification, identity_exchange::key_exchange},
};

pub async fn init_receiver() -> io::Result<()> {
    let identification = Identification::new();
    let mut udp = Connection::<Udp>::new_listen().await?;

    let discoverer_msg = loop {
        let Some(socket_addr) = Message::receive::<DiscoveryMessage>(&mut udp).await? else {
            continue;
        };
        break socket_addr;
    };

    //connect to it
    let mut tcp_connection = Connection::<Tcp>::new_send_n_listen(discoverer_msg.socket(), udp).await?;

    //read_host
    let host = Message::receive::<HostInfo>(&mut tcp_connection)
        .await?
        .ok_or_else(|| Error::new(io::ErrorKind::InvalidData, "invalid host info"))?;

    //confirm if user wants to reciecve data
    confirm_connection(host, &mut tcp_connection).await?;

    //exchange keys
    let identity_context = key_exchange(&mut tcp_connection, identification).await?;

    //check if the recieved pubident hash matches the actual key exchange identity
    identity_context.verify_identifiy(discoverer_msg)?;
    
    let _secret = init_reciever_key_exchange(&mut tcp_connection, &identity_context).await?;

    Ok(())
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
