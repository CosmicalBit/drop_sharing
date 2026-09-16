use tokio::io;

use crate::discovery::{
    lib::{Connection, Tcp, Udp},
    message::Message,
};

pub async fn init_reciever() -> io::Result<()> {
    //TODO:recieve the msg addr

    let udp = Connection::<Udp>::new_listen().await?;

    let socket_addr = loop {
        let Some(socket_addr) = Message::deserialize_socket_addr(&udp).await? else {
            continue;
        };
        break socket_addr;
    };

    //connect to it
    let _tcp_connection = Connection::<Tcp>::new_send_n_listen(socket_addr, udp).await?;

    //TODO: go to sender and send the hostname

    Ok(())
}
