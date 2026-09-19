//! contains the init function for the sender
use tokio::io;

use crate::{
    discovery::{
        connection::{Connection, Send, Serialize, Tcp, Udp},
        hostinfo::HostInfo,
        message::{
            Message,
            TransferResponse,
        },
    },
    encryption::key_agreement::sender::key_exchange::init_sender_key_exchange,
    identity::identity_exchange::key_exchange,
};

pub async fn sender_init(num_of_files: u32) -> io::Result<()> {
    let (_tcp_connection, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let msg = my_socket_addr.serialize();

    //send to udp
    Connection::<Udp>::start_broadcast_and_send(&msg).await?;


    let mut tcp_connection = Connection::<Tcp>::new(my_socket_addr).await?;

    //send host
    let host = &HostInfo::new(num_of_files)?.serialize();
    tcp_connection.send(host).await?;

    // Read receiver confirmation.
    let confirmation = Message::receive::<TransferResponse>(&mut tcp_connection)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "error parsing confirmation"))?;
    confirmation.confirm()?;

    //start ident exchange
    let identity_context = key_exchange(&mut tcp_connection).await?;
    let _secret = init_sender_key_exchange(&mut tcp_connection, &identity_context).await?;
    Ok(())
}
