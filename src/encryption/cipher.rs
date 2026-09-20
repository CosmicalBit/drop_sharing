use chacha20poly1305::{
    ChaCha20Poly1305, ChaChaPoly1305, Nonce,
    aead::{Aead, Payload},
};
use ml_dsa::{Generate, KeyInit};
use tokio::io;

use crate::{
    discovery::connection::Serialize, encryption::key_agreement::sender::keygen::Secret, file_collection::tree_walking::FileHeader,
};

#[derive(Debug)]
pub enum TransferError {
    Io(io::Error),
    Encrytion(chacha20poly1305::Error),
    InvalidChunkSize,
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

///when reading read buffer size + 16
impl Cipher {
    ///returns serialized header plus the encrypted contents with the nonce at the end
    pub fn encrypt_with_header(&self, buf: &[u8], header: &FileHeader) -> Result<(Box<[u8]>, Box<[u8]>), TransferError> {
        let header = header.serialize();
        let nonce = Nonce::generate();

        let payload = Payload { msg: buf, aad: &header };

        let mut encrypted = self.encryption_key.encrypt(&nonce, payload)?;
        encrypted.extend_from_slice(nonce.as_slice());

        Ok((header, encrypted.into_boxed_slice()))
    }
    pub fn encrypt(&self, buf: &[u8]) -> Result<Box<[u8]>, TransferError> {
        let nonce = Nonce::generate();

        let mut encrypted = self.encryption_key.encrypt(&nonce, buf)?;
        encrypted.extend_from_slice(nonce.as_slice());

        Ok(encrypted.into_boxed_slice())
    }

    pub fn decrypt(&self, encrypted: &[u8], aad: &[u8]) -> Result<Box<[u8]>, TransferError> {
        const NONCE_SIZE: usize = 12;

        let nonce_offset = encrypted.len().checked_sub(NONCE_SIZE).ok_or(TransferError::InvalidChunkSize)?;
        let (ciphertext, nonce_bytes) = encrypted.split_at(nonce_offset);
        let nonce = Nonce::try_from(nonce_bytes).map_err(|_| TransferError::InvalidChunkSize)?;
        let payload = Payload { msg: ciphertext, aad };
        let decrypted = self.encryption_key.decrypt(&nonce, payload)?;

        Ok(decrypted.into_boxed_slice())
    }
}
