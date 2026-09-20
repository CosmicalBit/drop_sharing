//! contains the init function for the sender
use tokio::io;

use crate::{
    discovery::{
        connection::{Connection, Send, Serialize, Tcp, Udp},
        hostinfo::HostInfo,
        message::{Message, TransferResponse},
        udp_logic::DiscoveryMessage,
    },
    encryption::key_agreement::sender::{key_exchange::init_sender_key_exchange, keygen::Secret},
    identity::{identity_definition::Identification, identity_exchange::key_exchange},
};

pub async fn sender_init(num_of_files: u32) -> io::Result<(Secret, Connection<Tcp>)> {
    let (listener, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let identification = Identification::new();

    let msg = DiscoveryMessage::new(my_socket_addr, &identification);
    //send to udp
    Connection::<Udp>::start_broadcast_and_send(&msg.serialize()).await?;

    let mut tcp_connection = Connection::<Tcp>::accept(listener).await?;

    //send host
    let host = &HostInfo::new(num_of_files)?.serialize();
    tcp_connection.send(host).await?;

    // Read receiver confirmation.
    let confirmation = Message::receive::<TransferResponse>(&mut tcp_connection).await?;
    confirmation.confirm()?;

    //start ident exchange
    let identity_context = key_exchange(&mut tcp_connection, identification).await?;
    let secret = init_sender_key_exchange(&mut tcp_connection, &identity_context).await?;

    Ok((secret, tcp_connection))
}
