//! this file implemts the necessary abstraction to 
//! extract Host related information sutch as [`Host`] and [`HostInfo`]

use std::io::Error;

use tokio::io;

use crate::discovery::connection::{Deserialize, IndicationBytes, Serialize, Size};

/// [`Host`] contains the computer host name
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Host {
    name: String,
}

impl Host {
    pub(crate) fn new() -> io::Result<Self> {
        let hostname = hostname::get()?;
        let name = hostname
            .to_str()
            .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidFilename, "non utf8 hostname"))?
            .to_string();

        Ok(Self { name })
    }
    #[cfg(test)]
    pub fn manual(name: &str) -> Self {
        Self { name: name.to_string() }
    }
}

impl Serialize for Host {
    fn serialize(&self) -> Vec<u8> {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u32;

        let mut vec = Vec::with_capacity(bytes.len() + 5);
        vec.push(IndicationBytes::HostName as u8);
        vec.extend_from_slice(&len.to_be_bytes());
        vec.extend_from_slice(bytes);
        vec
    }
}

//when host is decerialized we start by writting the [`IndicationBytes`] followed by the len witch is always a `u32`
impl Deserialize for Host {
    const SIZE: Size = Size::Dynamic {
        header_size: 5,
        total_size: |header| {
            if header.first().copied()? != IndicationBytes::HostName as u8 {
                return None;
            }
            let len = u32::from_be_bytes(header.get(1..5)?.try_into().ok()?) as usize;

            Some(5 + len)
        },
    };
    type Output = Self;
    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.first().copied()? != IndicationBytes::HostName as u8 {
            return None;
        }
        let len = u32::from_be_bytes(data.get(1..5)?.try_into().ok()?) as usize;

        Host::try_from(data.get(5..5 + len)?).ok()
    }
}
impl TryFrom<&[u8]> for Host {
    type Error = std::str::Utf8Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let name = std::str::from_utf8(value).to_owned()?;

        Ok(Self { name: name.to_string() })
    }
}

///[`HostInfo`] stores a [`Host`] plus the ammount of files that will be sent over `Tcp` later on
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct HostInfo {
    host: Host,
    num_of_files: u32,
}

impl HostInfo {
    pub fn new(num_of_files: u32) -> io::Result<Self> {
        let host = Host::new()?;
        Ok(Self { host, num_of_files })
    }
    pub fn name(&self) -> &str {
        &self.host.name
    }
    pub fn file_num(&self) -> usize {
        self.num_of_files as usize
    }
}

impl Serialize for HostInfo {
    fn serialize(&self) -> Vec<u8> {
        let name = &self.host.serialize();
        let len = name.len() as u32;
        let num_file_as_bytes = self.num_of_files.to_be_bytes();

        let mut vec = Vec::with_capacity(len as usize + num_file_as_bytes.len() + 3);

        vec.push(IndicationBytes::HostInfo as u8);

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
            let name_len = u32::from_be_bytes(header.get(1..5)?.try_into().ok()?) as usize;

            //header + name + num_of_Files
            Some(5 + name_len + 4)
        },
    };
    type Output = Self;

    fn deserialize(data: &[u8]) -> Option<Self>
    where
        Self: Sized,
    {
        if data[0] != IndicationBytes::HostInfo as u8 {
            return None;
        }

        let len = u32::from_be_bytes(data.get(1..5)?.try_into().ok()?);
        let host = Host::deserialize(data.get(5..5 + len as usize)?)?;
        let num_of_files = u32::from_be_bytes(data.get(5 + len as usize..9 + len as usize)?.try_into().ok()?);

        Some(HostInfo { host, num_of_files })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_host_info_round_trip() {
        let host_info = HostInfo::new(4).unwrap();

        let serilized = host_info.clone().serialize();

        let deserialized = HostInfo::deserialize(&serilized).unwrap();

        assert_eq!(host_info, deserialized);
    }
    #[test]
    fn test_host_round_trip() {
        let host = Host::new().unwrap();

        let seri = host.clone().serialize();

        let desiri = Host::deserialize(&seri).unwrap();

        assert_eq!(host, desiri);
    }
}
