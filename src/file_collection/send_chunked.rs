use std::{
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

use tokio::{
    fs,
    io::{self, AsyncWriteExt},
};

use crate::{
    discovery::connection::{Connection, DecodeError, IndicationBytes, Recieve, Send, Serialize},
    encryption::cipher::{Cipher, TransferError},
    file_collection::tree_walking::{CHUNCK_SIZE, FileHeader},
    ui::BackendCommunication,
};

const HEADER_PREFIX_SIZE: usize = 5;
const FILE_METADATA_SIZE: usize = 12;
const AUTH_TAG_SIZE: usize = 16;
const NONCE_SIZE: usize = 12;
const ENCRYPTION_OVERHEAD: usize = AUTH_TAG_SIZE + NONCE_SIZE;
const MAX_FILE_COUNT: usize = 100_000;
const MAX_FILE_NAME_SIZE: usize = 4_096;
const MAX_CHUNK_SIZE: usize = 1024 * 1024;

struct IncomingFile {
    path: PathBuf,
    header: Vec<u8>,
    file_size: u64,
    chunk_size: usize,
}

pub async fn send_files(
    headers: &[FileHeader],
    cipher: &Cipher,
    tcp: &mut Connection,
    backend: Option<&BackendCommunication>,
) -> Result<(), TransferError> {
    if headers.len() > MAX_FILE_COUNT {
        return Err(invalid_data(DecodeError::InvalidValue("too many files")).into());
    }

    let total_bytes = total_size(headers)?;
    let mut sent_bytes = 0u64;
    for header in headers {
        send_file(header, cipher, tcp, &mut sent_bytes, total_bytes, backend).await?;
    }

    if total_bytes == 0
        && let Some(backend) = backend
    {
        update_progress(total_bytes, sent_bytes, backend)?;
    }

    Ok(())
}
fn update_progress(total_bytes: u64, sent_bytes: u64, backend: &BackendCommunication) -> Result<(), TransferError> {
    let percentage = if total_bytes == 0 {
        1.0
    } else {
        (sent_bytes as f32 / total_bytes as f32).clamp(0.0, 1.0)
    };

    backend
        .progress
        .send(Some(percentage))
        .map_err(|error| io::Error::new(io::ErrorKind::BrokenPipe, error))?;
    Ok(())
}

async fn send_file(
    header: &FileHeader,
    cipher: &Cipher,
    tcp: &mut Connection,
    sent_bytes: &mut u64,
    total_bytes: u64,
    backend: Option<&BackendCommunication>,
) -> Result<(), TransferError> {
    if CHUNCK_SIZE == 0 || CHUNCK_SIZE > MAX_CHUNK_SIZE {
        return Err(TransferError::InvalidChunkSize);
    }

    //send header
    let mut file = File::open(header.path())?;
    let serialized_header = header.serialize();
    tcp.send(&serialized_header).await?;

    let chunk_size = u64::try_from(CHUNCK_SIZE)?;
    let chunk_count = header.file_size().div_ceil(chunk_size);
    let mut remaining = header.file_size();

    for chunk_index in 0..chunk_count {
        let current_size = remaining.min(chunk_size);
        let mut chunk = vec![0u8; usize::try_from(current_size)?];
        file.read_exact(&mut chunk)?;

        let aad = chunk_aad(&serialized_header, chunk_index);
        tcp.send(&chunk_index.to_be_bytes()).await?;
        tcp.send(&cipher.encrypt(&chunk, &aad)?).await?;
        remaining -= current_size;
        *sent_bytes += current_size;
        if let Some(backend) = backend {
            update_progress(total_bytes, *sent_bytes, backend)?;
        }
    }

    Ok(())
}

pub async fn collect_files(
    advertised_file_count: usize,
    output_directory: &Path,
    cipher: &Cipher,
    tcp: &mut Connection,
) -> Result<(), TransferError> {
    if advertised_file_count > MAX_FILE_COUNT {
        return Err(invalid_data(DecodeError::InvalidValue("file count is too large")).into());
    }

    fs::create_dir_all(output_directory).await?;

    for _ in 0..advertised_file_count {
        let incoming = receive_file_header(tcp).await?;
        receive_file(output_directory, incoming, cipher, tcp).await?;
    }

    Ok(())
}

async fn receive_file_header(tcp: &mut Connection) -> Result<IncomingFile, TransferError> {
    let mut header = vec![0u8; HEADER_PREFIX_SIZE];
    tcp.recieve(&mut header).await?;

    if header[0] != IndicationBytes::File as u8 {
        return Err(invalid_data(DecodeError::UnexpectedIndicationType {
            expected: IndicationBytes::File,
            actual: header[0],
        })
        .into());
    }

    let file_name_len = u32::from_be_bytes(header[1..5].try_into().expect("slice is 4 bytes")) as usize;
    if file_name_len == 0 || file_name_len > MAX_FILE_NAME_SIZE {
        return Err(invalid_data(DecodeError::InvalidValue("invalid file name length")).into());
    }

    let mut file_name = vec![0u8; file_name_len];
    tcp.recieve(&mut file_name).await?;
    header.extend_from_slice(&file_name);
    let file_name = std::str::from_utf8(&file_name)
        .map_err(DecodeError::InvalidUtf8)
        .map_err(invalid_data)?;
    let path = validate_relative_path(file_name)?;

    let mut metadata = [0u8; FILE_METADATA_SIZE];
    tcp.recieve(&mut metadata).await?;
    header.extend_from_slice(&metadata);
    let (file_size, chunk_size) = decode_file_metadata(&metadata)?;

    Ok(IncomingFile {
        path,
        header,
        file_size,
        chunk_size,
    })
}

async fn receive_file(output_directory: &Path, incoming: IncomingFile, cipher: &Cipher, tcp: &mut Connection) -> Result<(), TransferError> {
    let final_path = output_directory.join(&incoming.path);
    let parent = final_path
        .parent()
        .ok_or_else(|| invalid_data(DecodeError::InvalidValue("file has no parent directory")))?;

    fs::create_dir_all(parent).await?;
    let mut file = fs::File::create(final_path).await?;

    receive_chunks(&mut file, &incoming, cipher, tcp).await
}

async fn receive_chunks(file: &mut fs::File, incoming: &IncomingFile, cipher: &Cipher, tcp: &mut Connection) -> Result<(), TransferError> {
    let chunk_count = incoming.file_size.div_ceil(incoming.chunk_size as u64);
    let mut remaining = incoming.file_size;

    for chunk_index in 0..chunk_count {
        let mut received_index = [0u8; 8];
        tcp.recieve(&mut received_index).await?;
        let received_index = u64::from_be_bytes(received_index);
        if received_index != chunk_index {
            return Err(invalid_data(DecodeError::InvalidValue("chunk arrived out of order")).into());
        }

        let plaintext_size = remaining.min(incoming.chunk_size as u64) as usize;
        let mut encrypted = vec![0u8; plaintext_size + ENCRYPTION_OVERHEAD];
        tcp.recieve(&mut encrypted).await?;

        let aad = chunk_aad(&incoming.header, chunk_index);
        let decrypted = cipher.decrypt(&encrypted, &aad)?;
        if decrypted.len() != plaintext_size {
            return Err(invalid_data(DecodeError::InvalidLen {
                declared: plaintext_size,
                available: decrypted.len(),
            })
            .into());
        }

        file.write_all(&decrypted).await?;
        remaining -= plaintext_size as u64;
    }

    Ok(())
}

fn chunk_aad(header: &[u8], chunk_index: u64) -> Vec<u8> {
    let mut aad = Vec::with_capacity(header.len() + 8);
    aad.extend_from_slice(header);
    aad.extend_from_slice(&chunk_index.to_be_bytes());
    aad
}

fn decode_file_metadata(metadata: &[u8]) -> Result<(u64, usize), TransferError> {
    if metadata.len() != FILE_METADATA_SIZE {
        return Err(invalid_data(DecodeError::Truncated {
            expected: FILE_METADATA_SIZE,
            actual: metadata.len(),
        })
        .into());
    }

    let file_size = u64::from_be_bytes(metadata[..8].try_into().expect("slice is 8 bytes"));
    let chunk_size = u32::from_be_bytes(metadata[8..].try_into().expect("slice is 4 bytes")) as usize;
    if chunk_size == 0 || chunk_size > MAX_CHUNK_SIZE {
        return Err(TransferError::InvalidChunkSize);
    }

    Ok((file_size, chunk_size))
}

fn validate_relative_path(file_name: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(file_name);
    if path.as_os_str().is_empty() || path.components().any(|component| !matches!(component, Component::Normal(_))) {
        return Err(invalid_data(DecodeError::InvalidValue("invalid relative file path")));
    }

    Ok(path)
}

fn invalid_data(error: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

//count the total size
fn total_size(headers: &[FileHeader]) -> Result<u64, TransferError> {
    let mut size: u64 = 0;

    for header in headers {
        size = size.checked_add(header.file_size()).ok_or(TransferError::Math)?;
    }
    Ok(size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_safe_relative_paths() {
        assert!(validate_relative_path("photo.jpg").is_ok());
        assert!(validate_relative_path("folder/notes.txt").is_ok());
    }

    #[test]
    fn rejects_paths_outside_output_directory() {
        assert!(validate_relative_path("../secret.txt").is_err());
        assert!(validate_relative_path("/tmp/file.txt").is_err());
        assert!(validate_relative_path("folder/../file.txt").is_err());
    }

    #[test]
    fn decodes_file_size_and_chunk_size() {
        let mut metadata = Vec::new();
        metadata.extend_from_slice(&5_000_u64.to_be_bytes());
        metadata.extend_from_slice(&4_096_u32.to_be_bytes());

        assert_eq!(decode_file_metadata(&metadata).unwrap(), (5_000, 4_096));
    }

    #[test]
    fn chunk_aad_binds_the_chunk_order() {
        assert_ne!(chunk_aad(b"header", 0), chunk_aad(b"header", 1));
    }
}
