use std::{
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
};

use tokio::io;

use crate::discovery::connection::{IndicationBytes, Serialize};

const CHUNCK_SIZE: usize = 4096;

pub fn build_header(path: &Path) -> io::Result<Vec<FileHeader>> {
    let mut vec = Vec::with_capacity(20);
    collect_headers(path, path, &mut vec)?;

    Ok(vec)
}

fn collect_headers(root: &Path, path: &Path, headers: &mut Vec<FileHeader>) -> io::Result<()> {
    let dir = path.read_dir()?;

    for entry in dir {
        let entry = entry?;
        let ftype = entry.file_type()?;
        let path = entry.path();

        if ftype.is_dir() {
            collect_headers(root, &path, headers)?;
            continue;
        }

        if !ftype.is_file() {
            continue;
        }

        let size = entry.metadata()?.size();
        let relative_path = path
            .strip_prefix(root)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "couldnt get a relative file path"))?;
        let file_name = relative_path
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "file path isnt valid utf8"))?
            .to_owned();

        headers.push(FileHeader::new(path, file_name, size, CHUNCK_SIZE as u32));
    }

    Ok(())
}

///Doesnt implement [`Deserialize`] bcs it needs to be fully buffered
pub struct FileHeader {
    path: PathBuf,
    file_name: String,
    file_size: u64,
    chunk_size: u32,
}

impl FileHeader {
    pub fn new(path: PathBuf, file_name: String, file_size: u64, chunk_size: u32) -> Self {
        Self {
            file_name,
            path,
            file_size,
            chunk_size,
        }
    }
    pub fn file_size(&self) -> u64 {
        self.file_size
    }
    pub fn chunk_size(&self) -> u32 {
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
        vec.extend_from_slice(&self.chunk_size.to_be_bytes());

        vec.into_boxed_slice()
    }
}
