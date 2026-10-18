use crate::TransferError;
use postcard;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct Configs {
    pub download_dir: PathBuf,
}

fn configs_path() -> Result<PathBuf, TransferError> {
    Ok(dirs::config_dir()
        .ok_or(TransferError::Custom("coulndt find configs dir".to_string()))?
        .join("Drop_Sharing"))
}

impl Configs {
    pub fn set_default() -> Result<Self, TransferError> {
        let dir = dirs::download_dir().ok_or(TransferError::Custom("Couldnt find download dir".to_string()))?;

        let config = Self { download_dir: dir };

        config.save_config()?;

        Ok(config)
    }
    pub fn new() -> Result<Self, TransferError> {
        if configs_path()?.is_file() {
            let bytes = std::fs::read(configs_path()?)?;
            let config: Configs = match postcard::from_bytes(&bytes) {
                Ok(config) => config,
                Err(_) => Configs::set_default()?,
            };
            Ok(config)
        } else {
            Configs::set_default()
        }
    }

    pub fn save_config(&self) -> Result<(), TransferError> {
        let bytes = postcard::to_stdvec(&self).map_err(|_| TransferError::Custom("impossible to convert settings to bytes".to_string()))?;
        let path = configs_path()?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, bytes)?;

        Ok(())
    }
}
