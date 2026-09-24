//! contains the init function for the sender

use crate::{
    discovery::{
        connection::{Send, Serialize},
        hostinfo::HostInfo,
        message::{Message, TransferResponse},
    },
    encryption::{cipher::TransferError, key_agreement::sender::{key_exchange::init_sender_key_exchange, keygen::Secret}},
    identity::{identity_definition::Identification, identity_exchange::key_exchange},
    wpa::requests::{P2pConnection, sender_connect},
};

pub async fn sender_init(num_of_files: u32) -> Result<Option<(Secret, P2pConnection)>, TransferError> {
    let mut socket = sender_connect().await?;

    let identification = Identification::new();

    //send host
    let host = &HostInfo::new(num_of_files)?.serialize();
    socket.tcp.send(host).await?;

    // Read receiver confirmation.
    let confirmation = Message::receive::<TransferResponse>(&mut socket.tcp).await?;
    if !confirmation.accepted() {
        return Ok(None);
    }

    //start ident exchange
    let identity_context = key_exchange(&mut socket.tcp, identification).await?;
    let secret = init_sender_key_exchange(&mut socket.tcp, &identity_context).await?;

    Ok(Some((secret, socket)))
}
