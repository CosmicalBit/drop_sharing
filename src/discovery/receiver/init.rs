use std::{
    io::{Error, stdin},
    net::SocketAddr,
};

use tokio::io;

use crate::discovery::{
    hostinfo::HostInfo,
    lib::{Connection, Tcp, Udp},
    message::{Message, TransferDesision, TransferResponse},
};

pub async fn init_reciever() -> io::Result<()> {
    let mut udp = Connection::<Udp>::new_listen().await?;

    let socket_addr = loop {
        let Some(socket_addr) = Message::receive::<SocketAddr>(&mut udp).await? else {
            continue;
        };
        break socket_addr;
    };

    //connect to it
    let mut tcp_connection = Connection::<Tcp>::new_send_n_listen(socket_addr, udp).await?;

    //read_host
    let host = Message::receive::<HostInfo>(&mut tcp_connection)
        .await?
        .ok_or_else(|| Error::new(io::ErrorKind::InvalidData, "invalid host info"))?;

    //confirm if user wants to reciecve data
    confirm_connection(host, &mut tcp_connection).await?;

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
                Message::TransferResponse(TransferResponse::new(TransferDesision::Accepted)?)
                    .send(connection)
                    .await?;
                return Ok(());
            },
            "no" | "n" => {
                println!("you rejected, exiting program...");
                Message::TransferResponse(TransferResponse::new(TransferDesision::Rejected)?)
                    .send(connection)
                    .await?;
                return Err(io::Error::new(std::io::ErrorKind::InvalidData, "user decided to abort"));
            },
            _ => println!("please enter y or n"),
        }
    }
}
