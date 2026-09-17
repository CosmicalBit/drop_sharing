use std::{path::Path};

use tokio::io;

pub trait FileAction {
    fn action(&mut self, path: &Path) -> io::Result<()>;
}

pub fn walk<T: FileAction>(path: &Path, action: &mut T) -> io::Result<()> {
    recursive_dir(path, action)
}

fn recursive_dir(path: &Path, action: &mut impl FileAction) -> io::Result<()> {
    let dir = path.read_dir()?;

    for entry in dir {
        let entry = entry?;
        let ftype = entry.file_type()?;

        if ftype.is_dir() {
            recursive_dir(&entry.path(), action)?;
        }

        if ftype.is_file() {
            action.action(&entry.path())?;
        }
    }

    Ok(())
}

#[derive(Default)]
pub struct FileCount {
    count: usize,
}

impl FileAction for FileCount {
    fn action(&mut self, _path: &Path) -> io::Result<()> {
        self.count += 1;
        Ok(())
    }
}
impl FileCount{
    pub fn count(&self)-> usize{
        self.count
    }
}
