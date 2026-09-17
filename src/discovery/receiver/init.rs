use std::net::SocketAddr;

use tokio::io;

use crate::discovery::{
    hostinfo::HostInfo,
    lib::{Connection, Tcp, Udp},
    message::Message,
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
    Message::receive::<HostInfo>(&mut tcp_connection).await?;

    Ok(())
}
