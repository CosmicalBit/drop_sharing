use tokio::{
    io, net::{TcpListener, TcpSocket},
};
use std::net::SocketAddr;

use crate::discovery::lib::{Connection, Message, Serialize};




async fn sender_init() -> io::Result<()> {
    let (mut connection, my_socket_addr)  = Connection::new_listen().await?;
    
    
    let msg = Message::Address(my_socket_addr).serialize();

    connection.send(&msg);

    
    
    Ok(())
}

