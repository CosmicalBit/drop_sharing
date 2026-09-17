use std::{io::Error, process::Output};

use tokio::io;

use crate::discovery::lib::{Deserialize, IndicationBytes, Serialize, Size};

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct HostInfo {
    name: String,
    num_of_files: u32,
}
//todo get the num of files to be passedd to sender_init funciton and add to struct
//
impl HostInfo {
    pub fn new(num_of_files: u32) -> io::Result<Self> {
        let hostname = hostname::get()?;

        let hostname = hostname
            .to_str()
            .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidFilename, "non utf8 hostname"))?;

        Ok(Self {
            name: hostname.to_string(),
            num_of_files,
        })
    }
}

impl Serialize for HostInfo {
    fn serialize(self) -> Vec<u8> {
        let name = self.name.as_bytes();
        let len = name.len() as u32;
        let num_file_as_bytes = self.num_of_files.to_be_bytes();

        let mut vec = Vec::with_capacity(len as usize + num_file_as_bytes.len() + 3);

        vec.push(IndicationBytes::HostName as u8);

        vec.extend_from_slice(&len.to_be_bytes());
        vec.extend_from_slice(name);
        vec.extend_from_slice(&self.num_of_files.to_be_bytes());

        vec
    }
}

impl Deserialize for HostInfo {
    const SIZE: Size = Size::Dynamic {
        header_size: 5,
        total_size: |header| {
            let name_len = u32::from_be_bytes(header[1..5].try_into().ok()?) as usize;

            //header + name + num_of_Files
            Some(5 + name_len + 4)
        },
    };

    fn deserialize(data: &[u8]) -> Option<Self>
    where
        Self: Sized,
    {
        if data[0] != IndicationBytes::HostName as u8 {
            return None;
        }

        let len = u32::from_be_bytes(data[1..5].try_into().ok()?);
        let name = String::from_utf8_lossy(&data[5..5 + len as usize]).to_string();
        let num_of_files = u32::from_be_bytes(data[5 + len as usize..9 + len as usize].try_into().ok()?);

        Some(HostInfo { name, num_of_files })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_round_trip() {
        let host_info = HostInfo::new(4).unwrap();

        let serilized = host_info.clone().serialize();

        let deserialized = HostInfo::deserialize(&serilized).unwrap();

        assert_eq!(host_info, deserialized);
    }
}
