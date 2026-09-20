//! defines the main fucntion for encrytion key exchange on the reciever side
use ml_kem::ml_kem_1024::EncapsulationKey;
use tokio::io;

use crate::{
    discovery::{
        connection::{Recieve, SendSign, Serialize},
        message::Message,
    },
    encryption::key_agreement::sender::keygen::Secret,
    identity::identity_definition::IdentityContext,
};
pub(crate) async fn init_reciever_key_exchange(
    tcp_conn: &mut (impl Recieve + SendSign),
    identity_context: &IdentityContext,
) -> io::Result<Secret> {
    let encapsulation_key = Message::receive_signed::<EncapsulationKey>(tcp_conn, identity_context).await?;

    let secret = Secret::from(encapsulation_key);
    let serialized_secret = secret.serialize();

    tcp_conn.send_n_sign(&serialized_secret, identity_context).await?;

    Ok(secret)
}
