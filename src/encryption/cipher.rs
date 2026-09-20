use blake3::derive_key;
use chacha20poly1305::{ChaCha20Poly1305, ChaChaPoly1305, Nonce, aead::Aead};
use ml_dsa::{Generate, KeyInit};
use tokio::io;

use crate::encryption::key_agreement::sender::keygen::Secret;

pub enum TransferError {
    Io(io::Error),
    Encrytion(chacha20poly1305::Error),
}

impl From<io::Error> for TransferError {
    fn from(value: io::Error) -> Self {
        TransferError::Io(value)
    }
}
impl From<chacha20poly1305::Error> for TransferError {
    fn from(value: chacha20poly1305::Error) -> Self {
        TransferError::Encrytion(value)
    }
}

pub struct Cipher {
    encryption_key: ChaCha20Poly1305,
}

impl TryFrom<Secret> for Cipher {
    type Error = std::io::Error;
    fn try_from(value: Secret) -> Result<Self, Self::Error> {
        let chacha = ChaChaPoly1305::new_from_slice(value.secret())
            .map_err(|_| io::Error::new(std::io::ErrorKind::InvalidData, "ml-kem shared secret should be 32 bytes "))?;

        Ok(Self { encryption_key: chacha })
    }
}

impl Cipher {
    pub fn encrypt(&self, data: &[u8]) -> Result<(), TransferError> {
        let nounce = Nonce::generate();

        let encrypted = self.encryption_key.encrypt(&nounce, data)?;

        //TODO continue here and see if we want AED (aditional encrypted data)
        Ok(())
    }
}
