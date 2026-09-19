use tokio::io;

use crate::discovery::{
    hostinfo::HostInfo,
    lib::{Connection, Serialize, Tcp, Udp},
    message::{
        Message::{self},
        TransferResponse,
    },
};

pub async fn sender_init(num_of_files: u32) -> io::Result<()> {
    let (_tcp_connection, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let msg = Message::Address(my_socket_addr).serialize();

    //send to udp
    Connection::<Udp>::start_broadcast_and_send(&msg).await?;

    let host = Message::HostName(HostInfo::new(num_of_files)?);

    let mut tcp_connection = Connection::<Tcp>::new(my_socket_addr).await?;

    //send host
    host.send(&mut tcp_connection).await?;

    // Read receiver confirmation.
    let confirmation = Message::receive::<TransferResponse>(&mut tcp_connection)
        .await?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "error parsing confirmation"))?;
    confirmation.confirm()?;

    //start key agrrement
    Ok(())
}
