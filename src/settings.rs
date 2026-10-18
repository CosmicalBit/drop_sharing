use crate::TransferError;
use postcard;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, path::PathBuf};

#[derive(Serialize, Deserialize)]
pub struct Configs {
    pub recieve_dir: PathBuf,
}

fn configs_path() -> Result<PathBuf, TransferError> {
    Ok(dirs::config_dir()
        .ok_or(TransferError::Custom("coulndt find configs dir".to_string()))?
        .join("Drop_Sharing"))
}

impl Configs {
    fn new_hidden() -> Result<Self, TransferError> {
        let dir = dirs::download_dir().ok_or(TransferError::Custom("Couldnt find download dir".to_string()))?;

        let config = Self { recieve_dir: dir };

        config.save_config()?;

        Ok(config)
    }
    pub fn new() -> Result<Self, TransferError> {
        if configs_path()?.is_file() {
            let bytes = std::fs::read(configs_path()?)?;
            let config: Configs = postcard::from_bytes(&bytes).unwrap_or(Configs::new_hidden()?);
            Ok(config)
        } else {
            Configs::new_hidden()
        }
    }

    fn save_config(&self) -> Result<(), TransferError> {
        let bytes = postcard::to_stdvec(&self).map_err(|_| TransferError::Custom("impossible to convert settings to bytes".to_string()))?;
        fs::write(configs_path()?, bytes)?;

        Ok(())
    }
}
