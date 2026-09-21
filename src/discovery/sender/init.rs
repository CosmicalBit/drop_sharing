//! contains the init function for the sender
use std::time::Duration;

use tokio::{io, time::timeout};
const DISCOVERY_ATTEMPTS: usize = 5;
const DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

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
pub enum SenderInitError {
    Io(io::Error),
    Rejected,
}
impl From<io::Error> for SenderInitError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

pub async fn sender_init(num_of_files: u32) -> Result<(Secret, Connection<Tcp>), SenderInitError> {
    let (listener, my_socket_addr) = Connection::<Tcp>::new_listen().await?;

    let identification = Identification::new();

    let msg = DiscoveryMessage::new(my_socket_addr, &identification);
    //send to udp
    let serialized = msg.serialize();

    let mut tcp_connection = 'connected: {
        for _attempt in 0..DISCOVERY_ATTEMPTS {
            Connection::<Udp>::start_broadcast_and_send(&serialized).await?;
            match timeout(DISCOVERY_TIMEOUT, Connection::<Tcp>::accept(&listener)).await {
                Ok(result) => break 'connected result?,
                Err(_) => continue,
            }
        }

        return Err(io::Error::new(
            io::ErrorKind::TimedOut,
            "discovery timed out waiting for a receiver",
        ))?;
    };

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
