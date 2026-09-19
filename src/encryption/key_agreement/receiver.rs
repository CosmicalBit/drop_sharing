use std::io::ErrorKind;

use ml_kem::ml_kem_1024::EncapsulationKey;
use tokio::io;

use crate::{
    discovery::{
        connection::{Recieve, SendSign, Serialize},
        message::Message,
    },
    encryption::key_agreement::sender::keygen::Secret,
    identity::identity::IdentityContext,
};
pub(crate) async fn init_reciever_key_exchange(
    tcp_conn: &mut (impl Recieve + SendSign),
    identity_context: &IdentityContext,
) -> io::Result<Secret> {
    let encapsulation_key = Message::receive_signed::<EncapsulationKey>(tcp_conn, identity_context)
        .await?
        .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "invalid encapsulation key"))?;

    let secret = Secret::from(encapsulation_key);
    let serialized_secret = secret.serialize();

    tcp_conn.send_n_sign(&serialized_secret, identity_context).await?;

    Ok(secret)
}
