use std::io::ErrorKind;

use ml_kem::ml_kem_1024::Ciphertext;
use tokio::io;

use crate::{
    discovery::{
        connection::{Recieve, SendSign, Serialize},
        message::Message,
    },
    encryption::key_agreement::sender::keygen::{KeyPair, Secret},
    identity::identity::IdentityContext,
};

pub(crate) async fn init_sender_key_exchange(
    tcp_conn: &mut (impl Recieve + SendSign),
    identity_context: &IdentityContext,
) -> io::Result<Secret> {
    let sender_key_pair = KeyPair::generate_pair();
    let serial = sender_key_pair.serialize();

    tcp_conn.send_n_sign(&serial, identity_context).await?;

    let cyphertxt = Message::receive_signed::<Ciphertext>(tcp_conn, identity_context)
        .await?
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "invalid ciphertext"))?;

    Ok(sender_key_pair.construct_secret_from_keypair_n_cipher(cyphertxt))
}
