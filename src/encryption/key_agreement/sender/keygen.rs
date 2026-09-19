//! Encryption key generation and  exchane operations
//! 
//! this module provides the main helpers and abstracions for ecription key creation Deserialization and Serialization
//! the main structs are [`Secret`] and [`KeyPair`]
use ml_kem::{
    Decapsulate, Encapsulate, Kem, KeyExport, MlKem1024, TryKeyInit,
    kem::SharedKey,
    ml_kem_1024::{Ciphertext, DecapsulationKey, EncapsulationKey},
};
use zeroize::{Zeroize, Zeroizing};

use crate::discovery::connection::{
    Deserialize,
    IndicationBytes::{self},
    Serialize, Size,
};

///[`Secret`] contains the private encrytion key and the recieved / sent cyphertext
#[derive(PartialEq, Eq)]
pub struct Secret {
    secret: Zeroizing<SharedKey<MlKem1024>>,
    ciphertxt: Ciphertext,
}

impl Secret {
    fn from_ciphertxt(ciphertxt: Ciphertext, secret: DecapsulationKey) -> Self {
        let shared_secret = Zeroizing::new(secret.decapsulate(&ciphertxt));

        Self {
            secret: shared_secret,
            ciphertxt,
        }
    }
}

impl Serialize for Secret {
    ///serializes only the [`Ciphertext`] form this secret
    fn serialize(&self) -> Vec<u8> {
        Ciphertext::serialize(&self.ciphertxt)
    }
}

const CIPHERTXT_SIZE: usize = 1568;

impl Deserialize for Ciphertext {
    type Output = Self;
    const SIZE: Size = Size::Fixed(1 + CIPHERTXT_SIZE);
    fn deserialize(data: &[u8]) -> Option<Self> {
        if *data.first()? != IndicationBytes::Ciphertxt as u8 {
            return None;
        }

        let cipher = data.get(1..=CIPHERTXT_SIZE)?;

        Ciphertext::try_from(cipher).ok()
    }
}

impl Serialize for Ciphertext {
    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::Ciphertxt as u8);
        vec.extend_from_slice(self);

        vec
    }
}
impl KeyPair {
    ///cretes the shared secret based on  the already existing [`KeyPair`] and the recieved [`Ciphertext`]
    pub fn construct_secret_from_keypair_n_cipher(&self, cipher: Ciphertext) -> Secret {
        Secret::from_ciphertxt(cipher, self.secret())
    }
}

impl From<EncapsulationKey> for Secret {
    ///converts the recieved encapsulation key to [`Secret`]
    fn from(value: EncapsulationKey) -> Self {
        let (ciphertxt, mut secret) = value.encapsulate();

        let sharedsecret = Self {
            ciphertxt,
            secret: Zeroizing::new(secret),
        };

        //we need to be sure its set to zero
        secret.zeroize();

        sharedsecret
    }
}

///[`KeyPair`] contains the encapsolation and decapsulation keys which are essensial
/// for the ecrytion protocol and to obtain [`Secret`]
pub struct KeyPair {
    private_key: DecapsulationKey,
    public_key: EncapsulationKey,
}

const ML_KEN_PUB_1024_SIZE: usize = 1568;

impl Serialize for KeyPair {
    ///serializes only public key
    fn serialize(&self) -> Vec<u8> {
        self.public_key.serialize()
    }
}

impl Serialize for EncapsulationKey {
    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::PubKeySend as u8);
        vec.extend_from_slice(&self.to_bytes());

        vec
    }
}

impl Deserialize for EncapsulationKey {
    type Output = Self;
    //size of indication byte (u8) + the actual key size
    const SIZE: Size = Size::Fixed(1 + ML_KEN_PUB_1024_SIZE);
    ///creates [`EncapsulationKey`] based on the recieved public_key
    fn deserialize(data: &[u8]) -> Option<Self> {
        if *data.first()? != IndicationBytes::PubKeySend as u8 {
            return None;
        }

        let recieved_public_key = data.get(1..=ML_KEN_PUB_1024_SIZE)?;
        let recieved_public_key = EncapsulationKey::new_from_slice(recieved_public_key).ok()?;

        Some(recieved_public_key)
    }
}

impl KeyPair {
    pub fn generate_pair() -> Self {
        let (private_key, public_key) = MlKem1024::generate_keypair();

        KeyPair { private_key, public_key }
    }
    fn secret(&self) -> DecapsulationKey {
        self.private_key.clone()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn secret() {
        //sender side
        let sender_key_pair = KeyPair::generate_pair();
        let serial = sender_key_pair.serialize();

        //reciever side
        let decerial = EncapsulationKey::deserialize(&serial).unwrap();
        let recieve_secret = Secret::from(decerial);
        let serialized_secrset = recieve_secret.serialize();

        //sender side
        let cipher = Ciphertext::deserialize(&serialized_secrset).unwrap();
        let sender_secret = sender_key_pair.construct_secret_from_keypair_n_cipher(cipher);

        assert!(sender_secret == recieve_secret);
    }

    #[test]
    fn public_key_deserialization_rejects_empty_input() {
        assert!(EncapsulationKey::deserialize(&[]).is_none());
    }
}
