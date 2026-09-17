use crate::discovery::hostinfo::HostInfo;
use tokio::io;

use crate::discovery::message::Message::{self};
use crate::discovery::lib::{Connection,Udp, Serialize, Tcp};

pub async fn sender_init(num_of_files: u32) -> io::Result<()> {
    let (_tcp_connection, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let msg = Message::Address(my_socket_addr).serialize();

    //send to udp
    Connection::<Udp>::start_broadcast_and_send(&msg).await?;

    let host = Message::HostName(HostInfo::new(num_of_files)?);

    let mut tcp_connection = Connection::<Tcp>::new(my_socket_addr).await?;

    host.send(&mut tcp_connection).await?;
    
    Ok(())
}
