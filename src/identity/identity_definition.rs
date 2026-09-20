//! device identity and message identification
//!
//! the module provides the implementation for local device cryptographic identity

use ml_dsa::{Generate, KeyExport, KeyInit, Keypair, MlDsa87, Signature, Signer, SigningKey, Verifier, VerifyingKey};
use tokio::io;

use crate::discovery::{
    connection::{DecodeError, Deserialize, IndicationBytes, Serialize, Size},
    udp_logic::DiscoveryMessage,
};

const PUBLIC_KEY_SIZE: usize = 2592;
pub type PubKey = VerifyingKey<MlDsa87>;

///[`Identification`] contains the `SigningKey` and VerifyingKey`
pub struct Identification {
    private_key: SigningKey<MlDsa87>,
    public_key: VerifyingKey<MlDsa87>,
}

impl Identification {
    pub fn new() -> Self {
        let private = SigningKey::<MlDsa87>::generate();
        let public = private.verifying_key();

        Self {
            private_key: private,
            public_key: public,
        }
    }
    pub fn hash(&self) -> blake3::Hash {
        blake3::hash(&self.public_key.to_bytes())
    }
}

impl Serialize for Identification {
    fn serialize(&self) -> Box<[u8]> {
        self.public_key.serialize()
    }
}
impl Serialize for VerifyingKey<MlDsa87> {
    fn serialize(&self) -> Box<[u8]> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::PublicIdentKey as u8);
        vec.extend_from_slice(&self.to_bytes());

        vec.into_boxed_slice()
    }
}

impl Deserialize for Identification {
    type Output = PubKey;
    const SIZE: Size = Size::Fixed(1 + PUBLIC_KEY_SIZE);
    ///reads the public key of the other device
    fn deserialize(data: &[u8]) -> Result<Self::Output, DecodeError> {
        let indication = *data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })?;
        if indication != IndicationBytes::PublicIdentKey as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::PublicIdentKey,
                actual: indication,
            });
        }

        let verifying_key = data.get(1..=PUBLIC_KEY_SIZE).ok_or(DecodeError::Truncated {
            expected: 1 + PUBLIC_KEY_SIZE,
            actual: data.len(),
        })?;
        VerifyingKey::<MlDsa87>::new_from_slice(verifying_key).map_err(|_| DecodeError::InvalidValue("invalid ML-DSA public key"))
    }
}

/// [`IdentityContext`] contains the user [`Identification`] and its peer public key
pub struct IdentityContext {
    my_ident: Identification,
    peer_ident: PubKey,
}

pub const SIGNATURE_ADDED_SIZE: usize = 4627;
impl IdentityContext {
    pub fn new(my_ident: Identification, peer_ident: PubKey) -> Self {
        Self { my_ident, peer_ident }
    }
    pub fn sign(&self, data: &[u8]) -> Signature<MlDsa87> {
        self.my_ident.private_key.sign(data)
    }
    pub fn check_signature<'a>(&self, data: &'a [u8]) -> io::Result<&'a [u8]> {
        if data.len() < SIGNATURE_ADDED_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "signed message is shorter than an ML-DSA-87 signature",
            ));
        }

        let (data, signature) = data.split_at(data.len() - SIGNATURE_ADDED_SIZE);
        let signature = Signature::<MlDsa87>::try_from(signature)
            .map_err(|_err| io::Error::new(io::ErrorKind::InvalidData, "couldnt conver signature"))?;

        self.peer_ident
            .verify(data, &signature)
            .map_err(|_err| io::Error::new(io::ErrorKind::InvalidData, "detected the wrong signature"))?;

        Ok(data)
    }

    ///check if the recieved [`Identification`] hash matches the actual key exchange [`DiscoveryMessage`] finger_print
    pub fn verify_identifiy(&self, proclamed_identity: DiscoveryMessage) -> io::Result<()> {
        let veri_key_bytes = self.peer_ident.to_bytes();

        let hash = blake3::hash(&veri_key_bytes);

        if hash != proclamed_identity.figer_print() {
            return Err(io::Error::new(
                std::io::ErrorKind::InvalidData,
                "peer identity didint match discovery identity",
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use ml_dsa::SignatureEncoding;

    use super::*;

    #[test]
    fn ident_key_round_trip() {
        //sender
        let senderk = Identification::new();
        let sender_serial = senderk.serialize();

        //reciever
        let recieverk = Identification::new();
        let seder_k_stored_in_reciever = Identification::deserialize(&sender_serial).unwrap();
        let reciever_serial = recieverk.serialize();

        //sender
        let recieved_from_reciever_stored_in_sender = Identification::deserialize(&reciever_serial).unwrap();

        assert_eq!(recieved_from_reciever_stored_in_sender, recieverk.public_key);
        assert_eq!(seder_k_stored_in_reciever, senderk.public_key);
    }

    #[test]
    fn rejects_truncated_signed_message_without_panicking() {
        let sender = Identification::new();
        let receiver = Identification::new();
        let context = IdentityContext::new(receiver, sender.public_key);

        assert!(context.check_signature(b"too short").is_err());
    }

    #[test]
    fn rejects_tampered_signed_message() {
        let sender = Identification::new();
        let signature = sender.private_key.sign(b"authentic").to_bytes();
        let receiver = Identification::new();
        let context = IdentityContext::new(receiver, sender.public_key);
        let mut signed = b"authentic".to_vec();
        signed.extend_from_slice(&signature);

        signed[0] ^= 1;

        assert!(context.check_signature(&signed).is_err());
    }
}
