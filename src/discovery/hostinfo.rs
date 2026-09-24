//! this file implemts the necessary abstraction to
//! extract Host related information sutch as [`Host`] and [`HostInfo`]

use std::io::Error;

use tokio::io;

use crate::discovery::connection::{DecodeError, Deserialize, IndicationBytes, Serialize, Size};

const MAX_HOST_NAME_LEN: usize = 256;
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
}

impl Serialize for Host {
    fn serialize(&self) -> Box<[u8]> {
        let bytes = self.name.as_bytes();
        let len = bytes.len() as u32;

        let mut vec = Vec::with_capacity(bytes.len() + 5);
        vec.push(IndicationBytes::HostName as u8);
        vec.extend_from_slice(&len.to_be_bytes());
        vec.extend_from_slice(bytes);

        vec.into_boxed_slice()
    }
}

//when host is decerialized we start by writting the [`IndicationBytes`] followed by the len witch is always a `u32`
impl Deserialize for Host {
    const SIZE: Size = Size::Dynamic {
        header_size: 5,
        total_size: |header| {
            let indication = header.first().ok_or(DecodeError::Truncated {
                expected: 1,
                actual: header.len(),
            })?;

            if *indication != IndicationBytes::HostName as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::HostName,
                    actual: *indication,
                });
            }
            let len_bytes = header.get(1..5).ok_or(DecodeError::Truncated {
                expected: 5,
                actual: header.len(),
            })?;

            let len = u32::from_be_bytes(len_bytes.try_into().expect("1..5 is exacly 4 bytes")) as usize;

            Ok(5 + len)
        },
    };
    type Output = Self;
    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        let indication = data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })?;

        if *indication != IndicationBytes::HostName as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::HostName,
                actual: *indication,
            });
        }

        let len_bytes = data.get(1..5).ok_or(DecodeError::Truncated {
            expected: 5,
            actual: data.len(),
        })?;

        let len = u32::from_be_bytes(len_bytes.try_into().expect("1..5 always contains 4 bytes")) as usize;

        let name = data.get(5..5 + len).ok_or(DecodeError::InvalidLen {
            declared: len,
            available: data.len().saturating_sub(5),
        })?;

        Ok(Host::try_from(name)?)
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
    fn serialize(&self) -> Box<[u8]> {
        let name = &self.host.serialize();
        let len = name.len() as u32;
        let num_file_as_bytes = self.num_of_files.to_be_bytes();

        let mut vec = Vec::with_capacity(len as usize + num_file_as_bytes.len() + 3);

        vec.push(IndicationBytes::HostInfo as u8);

        vec.extend_from_slice(&len.to_be_bytes());
        vec.extend_from_slice(name);
        vec.extend_from_slice(&self.num_of_files.to_be_bytes());

        vec.into_boxed_slice()
    }
}

impl Deserialize for HostInfo {
    const SIZE: Size = Size::Dynamic {
        header_size: 5,
        total_size: |header| {
            let indication = header.first().ok_or(DecodeError::Truncated {
                expected: 1,
                actual: header.len(),
            })?;

            if *indication != IndicationBytes::HostInfo as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::HostInfo,
                    actual: *indication,
                });
            }

            let len_bytes = header.get(1..5).ok_or(DecodeError::InvalidLen {
                declared: 4,
                available: header.len(),
            })?;

            let name_len = u32::from_be_bytes(len_bytes.try_into().expect("1..5 is 4 bytes")) as usize;

            if name_len == 0 || name_len > MAX_HOST_NAME_LEN {
                return Err(DecodeError::InvalidValue("invalid len"));
            }
            //header + name + num_of_Files
            Ok(5 + name_len + 4)
        },
    };
    type Output = Self;

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        let indication = data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })?;
        if *indication != IndicationBytes::HostInfo as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::HostInfo,
                actual: *indication,
            });
        }

        let len_bytes = data.get(1..5).ok_or(DecodeError::Truncated {
            expected: 5,
            actual: data.len(),
        })?;
        let len = u32::from_be_bytes(len_bytes.try_into().expect("1..5 is exactly 4 bytes")) as usize;
        let payload_end = 5usize.checked_add(len).ok_or(DecodeError::InvalidLen {
            declared: len,
            available: data.len().saturating_sub(5),
        })?;
        let host_bytes = data.get(5..payload_end).ok_or(DecodeError::InvalidLen {
            declared: len,
            available: data.len().saturating_sub(5),
        })?;
        let host = Host::deserialize(host_bytes)?;
        let file_count_end = payload_end.checked_add(4).ok_or(DecodeError::InvalidLen {
            declared: len + 4,
            available: data.len().saturating_sub(5),
        })?;
        let file_count_bytes = data.get(payload_end..file_count_end).ok_or(DecodeError::Truncated {
            expected: file_count_end,
            actual: data.len(),
        })?;
        let num_of_files = u32::from_be_bytes(file_count_bytes.try_into().expect("file count is exactly 4 bytes"));

        Ok(HostInfo { host, num_of_files })
    }
}

#[cfg(test)]
mod test {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    use proptest::prelude::*;

    use super::*;

    impl Host {
        fn new_raw(bytes: &[u8]) -> io::Result<Host> {
            let name = OsStr::from_bytes(bytes);

            let name = name
                .to_str()
                .ok_or_else(|| Error::new(std::io::ErrorKind::InvalidFilename, "non utf8 hostname"))?
                .to_string();

            Ok(Host { name })
        }
    }
    impl HostInfo {
        fn new_raw(name_bytes: &[u8], num_of_files: u32) -> io::Result<HostInfo> {
            Ok(HostInfo {
                num_of_files,
                host: Host::new_raw(name_bytes)?,
            })
        }
    }

    #[test]
    fn fuzz_host_info() {
        bolero::check!().with_type::<(Vec<u8>, u32)>().for_each(|(name, num_of_files)| {
            let Ok(host_info) = HostInfo::new_raw(name, *num_of_files) else {
                return;
            };

            let seri = host_info.serialize();
            let deseri = HostInfo::deserialize(&seri).expect("it can never error if valid data");

            assert_eq!(host_info, deseri);
        })
    }

    fn host_info_strategy() -> impl Strategy<Value = HostInfo> {
        ("[a-zA-Z0-9-{1,63]", any::<u32>()).prop_map(|(name, num_of_files)| HostInfo {
            host: Host { name },
            num_of_files,
        })
    }

    proptest! {
        #[test]
        fn host_info_round_trip(host_info in host_info_strategy()){
            let serialized = host_info.serialize();
            let deserialized = HostInfo::deserialize(&serialized).unwrap();
            prop_assert_eq!(host_info, deserialized);
        }
    }
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
