use std::io;

use crate::{
    discovery::{
        connection::{Recieve, Send, Serialize},
        message::Message,
    },
    identity::identity_definition::{Identification, IdentityContext},
};

///Does the key exchange and returns [`IdentityContext`]
pub async fn key_exchange(tcp: &mut (impl Send + Recieve), my_identification: Identification) -> io::Result<IdentityContext> {
    //init sender_ident

    //send and recieve normally bcs until now theres no [`IdentityContext`]
    //send sender ident
    tcp.send(&my_identification.serialize()).await?;

    let pubkey = Message::receive::<Identification>(tcp).await?;

    Ok(IdentityContext::new(my_identification, pubkey))
}

#[cfg(test)]
mod tests {
    use ml_dsa::SignatureEncoding;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, DuplexStream},
        time::{Duration, timeout},
    };

    use super::*;
    use crate::{
        discovery::connection::SendSign,
        encryption::key_agreement::{receiver::init_reciever_key_exchange, sender::key_exchange::init_sender_key_exchange},
    };

    struct TestConnection(DuplexStream);

    impl Send for TestConnection {
        async fn send(&mut self, buffer: &[u8]) -> io::Result<()> {
            self.0.write_all(buffer).await
        }
    }

    impl Recieve for TestConnection {
        async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()> {
            self.0.read_exact(buffer).await.map(|_| ())
        }
    }

    impl SendSign for TestConnection {
        async fn send_n_sign(&mut self, buffer: &[u8], identity: &IdentityContext) -> io::Result<()> {
            let signature = identity.sign(buffer).to_bytes();
            self.0.write_all(buffer).await?;
            self.0.write_all(&signature).await
        }
    }

    fn connected_pair() -> (TestConnection, TestConnection) {
        let (left, right) = tokio::io::duplex(32 * 1024);
        (TestConnection(left), TestConnection(right))
    }

    #[tokio::test]
    async fn identity_exchange_authenticates_key_agreement() {
        timeout(Duration::from_secs(10), async {
            let (mut sender, mut receiver) = connected_pair();

            let sender_identification = Identification::new();
            let receiver_identification = Identification::new();
            let (sender_identity, receiver_identity) = tokio::try_join!(
                key_exchange(&mut sender, sender_identification),
                key_exchange(&mut receiver, receiver_identification),
            )?;

            let (sender_secret, receiver_secret) = tokio::try_join!(
                init_sender_key_exchange(&mut sender, &sender_identity),
                init_reciever_key_exchange(&mut receiver, &receiver_identity),
            )?;

            assert!(sender_secret == receiver_secret);
            io::Result::Ok(())
        })
        .await
        .expect("identity and authenticated key exchange timed out")
        .expect("identity and authenticated key exchange failed");
    }
}
