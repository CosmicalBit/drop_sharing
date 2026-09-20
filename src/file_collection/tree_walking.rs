use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use chacha20poly1305::Nonce;
use ml_dsa::Generate;
use tokio::io;

use crate::discovery::connection::{IndicationBytes, Serialize};

const CHUNCK_SIZE: usize = 4096;
pub const NONCE_SIZE : u32 = 12;

pub fn build_header(path: &Path) -> io::Result<Vec<FileHeader>> {
    let dir = path.read_dir()?;

    let mut vec = Vec::with_capacity(20);

    for entry in dir {
        let entry = entry?;
        let ftype = entry.file_type()?;

        if ftype.is_dir() {
            let Ok(_) = build_header(&entry.path()) else {
                continue;
            };
        }

        if !ftype.is_file() {
            continue;
        }

        let path = entry.path();
        let size = entry.metadata()?.size();

        let nonce = Nonce::generate();
        let header = FileHeader::new(path, size, CHUNCK_SIZE as u32, nonce)?;

        vec.push(header);
    }

    Ok(vec)
}

///Doesnt implement [`Deserialize`] bcs it needs to be fully buffered
pub struct FileHeader {
    path: PathBuf,
    file_name: String,
    file_size: u64,
    chunk_size: u32,
}

impl FileHeader {
    pub fn new(path: PathBuf, file_size: u64, chunk_size: u32, _nonce: Nonce) -> io::Result<Self> {
        Ok(Self {
            file_name: path
                .file_name()
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "couldnt get a file name"))?
                .to_string_lossy()
                .into_owned(),
            path,
            file_size,
            chunk_size,
        })
    }
    pub fn file_size(&self) -> u64{
        self.file_size
    }
    pub fn chunk_size(&self)-> u32{
        self.chunk_size
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
}
impl Serialize for FileHeader {
    fn serialize(&self) -> Box<[u8]> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::File as u8);
        vec.extend_from_slice(&(self.file_name.len() as u32).to_be_bytes());
        vec.extend_from_slice(self.file_name.as_bytes());
        vec.extend_from_slice(&(self.file_size).to_be_bytes());
        vec.extend_from_slice(&(self.chunk_size + NONCE_SIZE ).to_be_bytes());

        vec.into_boxed_slice()
    }
}
