//! this module provides mainly [`Message`] to have an abstraction to recieving anny data type over network
//!
//! it also defines [`TransferDesision`] and [`TransferResponse`] for network communication and explicitnessa acception
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use strum::EnumIter;

use crate::{
    discovery::{
        connection::{
            DecodeError, Deserialize,
            IndicationBytes::{self},
            Recieve, Serialize, Size,
        },
        hostinfo::Host,
    },
    identity::identity_definition::{IdentityContext, SIGNATURE_ADDED_SIZE},
};

///[`Message`] is used to be a generic stateless helper for recieving data over the network
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Message;

/// [`TransferDesision`] is used to represent if a transfier was accepted or no
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Debug, EnumIter)]
pub enum TransferDesision {
    Accepted = 1,
    Rejected = 0,
}

impl Serialize for TransferDesision {
    fn serialize(&self) -> Box<[u8]> {
        vec![self.clone() as u8].into_boxed_slice()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TransferResponse {
    host: Host,
    decision: TransferDesision,
}
impl TransferResponse {
    pub fn confirm(&self) -> io::Result<()> {
        // TODO: maybe ask the user for confirmatio
        match self.decision {
            TransferDesision::Accepted => Ok(()),
            TransferDesision::Rejected => Err(io::Error::new(io::ErrorKind::InvalidData, "transfer rejectedd")),
        }
    }
}

impl TransferResponse {
    pub fn new(decision: TransferDesision) -> io::Result<Self> {
        let host = Host::new()?;

        Ok(Self { host, decision })
    }
}

impl Serialize for TransferResponse {
    fn serialize(&self) -> Box<[u8]> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::TransferResponse as u8);
        vec.extend_from_slice(&self.host.serialize());
        vec.extend_from_slice(&self.decision.serialize());

        vec.into_boxed_slice()
    }
}
impl Deserialize for TransferDesision {
    const SIZE: Size = Size::Fixed(1);
    type Output = Self;

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        const ACCEPT: u8 = TransferDesision::Accepted as u8;
        const REJECT: u8 = TransferDesision::Rejected as u8;

        match *data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })? {
            ACCEPT => Ok(TransferDesision::Accepted),
            REJECT => Ok(TransferDesision::Rejected),
            _ => Err(DecodeError::InvalidValue("invalid transfer decision")),
        }
    }
}

impl Deserialize for TransferResponse {
    type Output = Self;
    const SIZE: Size = Size::Dynamic {
        header_size: 6,
        total_size: |header| {
            let indication = *header.first().ok_or(DecodeError::Truncated {
                expected: 1,
                actual: header.len(),
            })?;
            if indication != IndicationBytes::TransferResponse as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::TransferResponse,
                    actual: indication,
                });
            }
            let host_indication = *header.get(1).ok_or(DecodeError::Truncated {
                expected: 2,
                actual: header.len(),
            })?;
            if host_indication != IndicationBytes::HostName as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::HostName,
                    actual: host_indication,
                });
            }
            let len_bytes = header.get(2..6).ok_or(DecodeError::Truncated {
                expected: 6,
                actual: header.len(),
            })?;
            let host_len = u32::from_be_bytes(len_bytes.try_into().expect("2..6 is exactly 4 bytes")) as usize;
            Ok(7 + host_len)
        },
    };

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        let indication = *data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })?;
        if indication != IndicationBytes::TransferResponse as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::TransferResponse,
                actual: indication,
            });
        }
        let len_bytes = data.get(2..6).ok_or(DecodeError::Truncated {
            expected: 6,
            actual: data.len(),
        })?;
        let host_len = u32::from_be_bytes(len_bytes.try_into().expect("2..6 is exactly 4 bytes")) as usize;
        let host_end = 6 + host_len;
        let host_bytes = data.get(1..host_end).ok_or(DecodeError::InvalidLen {
            declared: host_len,
            available: data.len().saturating_sub(6),
        })?;
        let host = Host::deserialize(host_bytes)?;
        let decision_bytes = data.get(host_end..host_end + 1).ok_or(DecodeError::Truncated {
            expected: host_end + 1,
            actual: data.len(),
        })?;
        let decision = TransferDesision::deserialize(decision_bytes)?;

        Ok(Self { host, decision })
    }
}

impl Message {
    pub async fn receive<T: Deserialize>(connection: &mut impl Recieve) -> io::Result<T::Output> {
        const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let total = total_size(&data).map_err(invalid_data)?;
                if total > MAX_MESSAGE_SIZE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "message is too large"));
                }

                data.resize(total, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        T::deserialize(&data).map_err(invalid_data)
    }

    pub async fn receive_signed<T: Deserialize>(
        connection: &mut impl Recieve,
        identity_context: &IdentityContext,
    ) -> io::Result<T::Output> {
        const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size + SIGNATURE_ADDED_SIZE];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let total = total_size(&data).map_err(invalid_data)?;
                if total > MAX_MESSAGE_SIZE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "message is too large"));
                }

                data.resize(total + SIGNATURE_ADDED_SIZE, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        let data = identity_context.check_signature(&data)?;

        T::deserialize(data).map_err(invalid_data)
    }
}

fn invalid_data(error: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

impl Serialize for SocketAddr {
    fn serialize(&self) -> Box<[u8]> {
        match self {
            SocketAddr::V4(addr) => {
                let mut out = Vec::with_capacity(20);

                out.push(IndicationBytes::MagicInit as u8);
                out.push(4); // ipv4
                out.extend_from_slice(&addr.ip().octets());
                out.extend_from_slice(&addr.port().to_be_bytes());
                out.extend_from_slice(&[0u8; 12]);

                out.into_boxed_slice()
            },
            SocketAddr::V6(addr) => {
                let mut out = Vec::with_capacity(20);

                out.push(IndicationBytes::MagicInit as u8);
                out.push(6); // ipv6
                out.extend_from_slice(&addr.ip().octets());
                out.extend_from_slice(&addr.port().to_be_bytes());
                out.into_boxed_slice()
            },
        }
    }
}

impl Deserialize for SocketAddr {
    type Output = Self;
    const SIZE: Size = Size::Fixed(20);

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() != 20 {
            return Err(DecodeError::Truncated {
                expected: 20,
                actual: data.len(),
            });
        }

        let ip = data[1];

        match ip {
            4 => {
                //ipv4
                let ip = Ipv4Addr::new(data[2], data[3], data[4], data[5]);
                let port = u16::from_be_bytes([data[6], data[7]]);

                Ok(SocketAddr::V4(SocketAddrV4::new(ip, port)))
            },
            6 => {
                //ipv6
                let mut ip = [0u8; 16];
                ip.copy_from_slice(&data[2..18]);
                let port = u16::from_be_bytes([data[18], data[19]]);
                let ip = Ipv6Addr::from_octets(ip);

                Ok(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0)))
            },
            _ => Err(DecodeError::InvalidValue("unsupported IP version")),
        }
    }
}

#[cfg(test)]
mod test {
    use strum::IntoEnumIterator;

    use super::*;

    fn msg_sckaddr() -> (SocketAddr, SocketAddr) {
        let ipv4: SocketAddr = "127.0.0.1:4242".parse().unwrap();
        let ipv6: SocketAddr = "[::1]:4242".parse().unwrap();

        (ipv4, ipv6)
    }

    #[test]
    fn check_deserialized_serialize_round_trip() {
        let (org_ipv4, ipv6) = msg_sckaddr();

        let serialized = org_ipv4.clone().serialize();

        let deserialized = SocketAddr::deserialize(&serialized).unwrap();

        assert_eq!(deserialized, org_ipv4);

        let serialized = ipv6.clone().serialize();

        let deserialized = SocketAddr::deserialize(&serialized).unwrap();

        assert_eq!(deserialized, ipv6);
    }
    #[test]
    fn round_trip_of_transfer_responce() {
        for decision in TransferDesision::iter() {
            let responce = TransferResponse::new(decision).unwrap();

            let seri = responce.clone().serialize();

            let deri = TransferResponse::deserialize(&seri).unwrap();

            assert_eq!(deri, responce);
        }
    }
}
