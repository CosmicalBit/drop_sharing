
use tokio::io;

use crate::discovery::message::Message;
use crate::discovery::lib::{Connection,Udp, Serialize, Tcp};

async fn sender_init() -> io::Result<()> {
    let (_tcp_connection, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let msg = Message::Address(my_socket_addr).serialize();

    //send to udp
     Connection::<Udp>::start_broadcast_and_send(&msg).await?;

    
    
    Ok(())
}
