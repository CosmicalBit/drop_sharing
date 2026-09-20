use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

use tokio::{
    fs,
    io::{self, AsyncWriteExt},
};

use crate::{
    discovery::connection::{Connection, DecodeError, IndicationBytes, Recieve, Send, Serialize, Tcp},
    encryption::cipher::{Cipher, TransferError},
    file_collection::tree_walking::FileHeader,
};

const HEADER_PREFIX_SIZE: usize = 5;
const FILE_METADATA_SIZE: usize = 12;
const AUTH_TAG_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;

pub async fn send_files(headers: &[FileHeader], cipher: &Cipher, tcp: &mut Connection<Tcp>) -> Result<(), TransferError> {
    for header in headers {
        let chunk_size = header.chunk_size();
        if chunk_size == 0 {
            return Err(TransferError::InvalidChunkSize);
        }

        let mut file = File::open(header.path())?;
        let serialized_header = header.serialize();
        tcp.send(&serialized_header).await?;

        let mut remaining = header.file_size();
        let mut first_chunk = true;
        while remaining > 0 {
            let current_size = remaining.min(chunk_size as u64) as usize;
            let mut chunk = vec![0u8; current_size];
            file.read_exact(&mut chunk)?;

            let encrypted = if first_chunk {
                cipher.encrypt_with_header(&chunk, header)?.1
            } else {
                cipher.encrypt(&chunk)?
            };
            tcp.send(&encrypted).await?;

            remaining -= current_size as u64;
            first_chunk = false;
        }
    }

    Ok(())
}

async fn get_file_name(buffer: &[u8], tcp: &mut Connection<Tcp>) -> io::Result<String> {
    let length_bytes = buffer
        .get(1..5)
        .ok_or(DecodeError::Truncated {
            expected: 5,
            actual: buffer.len(),
        })
        .map_err(invalid_data)?;
    let file_name_len = u32::from_be_bytes(length_bytes.try_into().expect("slice is 4 bytes")) as usize;
    let mut file_name_bytes = vec![0u8; file_name_len];
    tcp.recieve(&mut file_name_bytes).await?;

    std::str::from_utf8(&file_name_bytes)
        .map(str::to_owned)
        .map_err(|error| invalid_data(DecodeError::InvalidUtf8(error)))
}

pub async fn collect_files(
    file_count: usize,
    output_directory: &Path,
    cipher: &Cipher,
    tcp: &mut Connection<Tcp>,
) -> Result<(), TransferError> {
    fs::create_dir_all(output_directory).await?;

    for _ in 0..file_count {
        let mut header = vec![0u8; HEADER_PREFIX_SIZE];
        tcp.recieve(&mut header).await?;

        let indication = header[0];
        if indication != IndicationBytes::File as u8 {
            return Err(invalid_data(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::File,
                actual: indication,
            })
            .into());
        }

        let file_name = get_file_name(&header, tcp).await?;
        validate_file_name(&file_name)?;
        header.extend_from_slice(file_name.as_bytes());

        let mut metadata = [0u8; FILE_METADATA_SIZE];
        tcp.recieve(&mut metadata).await?;
        header.extend_from_slice(&metadata);

        let (file_size, chunk_size) = decode_file_metadata(&metadata)?;

        let mut file = fs::File::create(output_directory.join(&file_name)).await?;
        let mut remaining = file_size;
        let mut first_chunk = true;

        while remaining > 0 {
            let plaintext_size = remaining.min(chunk_size as u64) as usize;
            let mut encrypted = vec![0u8; plaintext_size + AUTH_TAG_SIZE + NONCE_SIZE];
            tcp.recieve(&mut encrypted).await?;

            let aad = if first_chunk { header.as_slice() } else { &[] };
            let decrypted = cipher.decrypt(&encrypted, aad)?;
            if decrypted.len() != plaintext_size {
                return Err(invalid_data(DecodeError::InvalidLen {
                    declared: plaintext_size,
                    available: decrypted.len(),
                })
                .into());
            }

            file.write_all(&decrypted).await?;
            remaining -= plaintext_size as u64;
            first_chunk = false;
        }
    }

    Ok(())
}

fn decode_file_metadata(metadata: &[u8]) -> Result<(u64, usize), TransferError> {
    let file_size_bytes = metadata.get(..8).ok_or_else(|| {
        invalid_data(DecodeError::Truncated {
            expected: FILE_METADATA_SIZE,
            actual: metadata.len(),
        })
    })?;
    let chunk_size_bytes = metadata.get(8..FILE_METADATA_SIZE).ok_or_else(|| {
        invalid_data(DecodeError::Truncated {
            expected: FILE_METADATA_SIZE,
            actual: metadata.len(),
        })
    })?;

    let file_size = u64::from_be_bytes(file_size_bytes.try_into().expect("slice is 8 bytes"));
    let transmitted_chunk_size = u32::from_be_bytes(chunk_size_bytes.try_into().expect("slice is 4 bytes")) as usize;
    let chunk_size = transmitted_chunk_size
        .checked_sub(NONCE_SIZE)
        .filter(|size| *size > 0)
        .ok_or(TransferError::InvalidChunkSize)?;

    Ok((file_size, chunk_size))
}

fn validate_file_name(file_name: &str) -> io::Result<()> {
    let path = PathBuf::from(file_name);
    if path.file_name().and_then(|name| name.to_str()) != Some(file_name) {
        return Err(invalid_data(DecodeError::InvalidValue("invalid file name")));
    }

    Ok(())
}

fn invalid_data(error: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_file_names() {
        assert!(validate_file_name("photo.jpg").is_ok());
        assert!(validate_file_name("notes 2026.txt").is_ok());
    }

    #[test]
    fn rejects_paths_instead_of_file_names() {
        assert!(validate_file_name("../secret.txt").is_err());
        assert!(validate_file_name("folder/file.txt").is_err());
        assert!(validate_file_name("/tmp/file.txt").is_err());
    }

    #[test]
    fn decodes_file_size_and_plaintext_chunk_size() {
        let mut metadata = Vec::new();
        metadata.extend_from_slice(&5_000_u64.to_be_bytes());
        metadata.extend_from_slice(&4_108_u32.to_be_bytes());

        assert_eq!(decode_file_metadata(&metadata).unwrap(), (5_000, 4_096));
    }

    #[test]
    fn rejects_chunk_size_without_payload_space() {
        let mut metadata = Vec::new();
        metadata.extend_from_slice(&10_u64.to_be_bytes());
        metadata.extend_from_slice(&(NONCE_SIZE as u32).to_be_bytes());

        assert!(matches!(decode_file_metadata(&metadata), Err(TransferError::InvalidChunkSize)));
    }
}
