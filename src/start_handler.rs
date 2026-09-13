use smallvec::SmallVec;

use crate::arguments::StartArgs;
use std::ffi::OsStr;
use std::fs::{File, FileType};
use std::hint::select_unpredictable;
use std::io;
use std::path::Path;
use std::process::Output;
use std::{fs::read_dir, path::PathBuf};

fn hadle_start(args: StartArgs) -> io::Result<()> {
    let dir = args.directory;

    // send the confirmation if asked
    let file_info_list: Vec<FileInfo> = walk::<FileInfo>(&dir)?;

    Ok(())
}

fn walk<A: FileAction>(path: &Path) -> io::Result<Vec<A::Output>> {
    let mut vec = Vec::new();
    recursive_dir::<A>(path, &mut vec)?;

    Ok(vec)
}

///walks a dir and for every item that is a file it calls a function that inplemets [`FileAction`]
///
fn recursive_dir<A: FileAction>(path: &Path, out: &mut Vec<A::Output>) -> io::Result<()> {
    let dir = read_dir(path)?;

    for entry in dir {
        let entry = entry?;
        let entry_type = entry.file_type()?;
        let entry_name = entry.path();

        if entry_type.is_dir() {
            recursive_dir::<A>(&entry.path(), out)?;
        }

        //skip anything not a file
        if !entry_type.is_file() {
            continue;
        }

        //call action and store its result in the 'out' vector
        let action_result = A::action(&entry_name)?;

        out.push(action_result);
    }
    Ok(())
}

/// A simple trait to make it  difficult too call the wrong fucntion in [`recusive_dir`]
pub trait FileAction {
    type Output;
    fn action(path: &Path) -> io::Result<Self::Output>;
}

#[repr(u8)]
#[derive(Clone)]
enum FileKind {
    File = 1,
    Symlink = 2,
    Dir = 3,
    Other = 4,
}

impl From<FileType> for FileKind {
    fn from(value: FileType) -> Self {
        if value.is_dir() {
            Self::Dir
        } else if value.is_file() {
            Self::File
        } else if value.is_symlink() {
            Self::Symlink
        } else {
            Self::Other
        }
    }
}

///File info is used to be sent as a confirmation
/// its merely to check if the use  wants to recieve
// TODO: this needs to implement trait [`Packed`] to be sendable over network
pub struct FileInfo {
    name: String,
    path: PathBuf,
    kind: FileKind,
    size: u64,
}

impl FileInfo {
    pub fn new(name: String, path: &Path, size: u64, kind: FileKind) -> Self {
        Self {
            name,
            path: PathBuf::from(path),
            size,
            kind,
        }
    }
}

impl FileInfo {
    pub fn name(&self) -> &str {
        self.name.as_str()
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn kind(&self) -> u8 {
        self.kind.clone() as u8
    }
    pub fn size(&self) -> u64 {
        self.size
    }
}

impl FileAction for FileInfo {
    type Output = Self;

    fn action(path: &Path) -> io::Result<Self::Output>
    where
        Self: Sized,
    {
        let file = File::open(path)?;

        let metadata = file.metadata()?;

        let size = metadata.len();
        let path = path;
        let ftype = FileKind::from(metadata.file_type());
        let name = path.file_name().expect("invisible file name").to_str().unwrap().to_string();

        Ok(FileInfo::new(name, path, size, ftype))
    }
}
