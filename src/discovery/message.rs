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
            Deserialize,
            IndicationBytes::{self},
            Recieve, Serialize, Size,
        },
        hostinfo::Host,
    },
    identity::identity::{IdentityContext, SIGNATURE_ADDED_SIZE},
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
    fn serialize(&self) -> Vec<u8> {
        vec![self.clone() as u8]
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
    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::TransferResponse as u8);
        vec.extend_from_slice(&self.host.serialize());
        vec.extend_from_slice(&self.decision.serialize());
        vec
    }
}
impl Deserialize for TransferDesision {
    const SIZE: Size = Size::Fixed(1);
    type Output = Self;

    fn deserialize(data: &[u8]) -> Option<Self> {
        const ACCEPT: u8 = TransferDesision::Accepted as u8;
        const REJECT: u8 = TransferDesision::Rejected as u8;

        match *data.first()? {
            ACCEPT => Some(TransferDesision::Accepted),
            REJECT => Some(TransferDesision::Rejected),
            _ => None,
        }
    }
}

impl Deserialize for TransferResponse {
    type Output = Self;
    const SIZE: Size = Size::Dynamic {
        header_size: 6,
        total_size: |header| {
            if header.first().copied()? != IndicationBytes::TransferResponse as u8
                || header.get(1).copied()? != IndicationBytes::HostName as u8
            {
                return None;
            }

            let host_len = u32::from_be_bytes(header.get(2..6)?.try_into().ok()?) as usize;
            Some(7 + host_len)
        },
    };

    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.first().copied()? != IndicationBytes::TransferResponse as u8 {
            return None;
        }
        let host_len = u32::from_be_bytes(data.get(2..6)?.try_into().ok()?) as usize;
        let host_end = 6 + host_len;
        let host = Host::deserialize(data.get(1..host_end)?)?;
        let decision = TransferDesision::deserialize(data.get(host_end..host_end + 1)?)?;

        Some(Self { host, decision })
    }
}

impl Message {
    pub async fn receive<T: Deserialize>(connection: &mut impl Recieve) -> io::Result<Option<T::Output>> {
        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let Some(total) = total_size(&data) else {
                    return Ok(None);
                };

                data.resize(total, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        Ok(T::deserialize(&data))
    }

    pub async fn receive_signed<T: Deserialize>(
        connection: &mut impl Recieve,
        identity_context: &IdentityContext,
    ) -> io::Result<Option<T::Output>> {
        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size + SIGNATURE_ADDED_SIZE];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let Some(total) = total_size(&data) else {
                    return Ok(None);
                };

                data.resize(total + SIGNATURE_ADDED_SIZE, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        let data = identity_context.check_signature(&data)?;

        Ok(T::deserialize(data))
    }
}

impl Serialize for SocketAddr {
    fn serialize(&self) -> Vec<u8> {
        match self {
            SocketAddr::V4(addr) => {
                let mut out = Vec::with_capacity(20);

                out.push(IndicationBytes::MagicInit as u8);
                out.push(4); // ipv4
                out.extend_from_slice(&addr.ip().octets());
                out.extend_from_slice(&addr.port().to_be_bytes());
                out.extend_from_slice(&[0u8; 12]);

                out
            },
            SocketAddr::V6(addr) => {
                let mut out = Vec::with_capacity(20);

                out.push(IndicationBytes::MagicInit as u8);
                out.push(6); // ipv6
                out.extend_from_slice(&addr.ip().octets());
                out.extend_from_slice(&addr.port().to_be_bytes());
                out
            },
        }
    }
}


impl Deserialize for SocketAddr {
    type Output = Self;
    const SIZE: Size = Size::Fixed(20);

    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() != 20 {
            dbg!("deserialize socket addr is in wrong place");
            return None;
        }

        let ip = data[1];

        match ip {
            4 => {
                //ipv4
                let ip = Ipv4Addr::new(data[2], data[3], data[4], data[5]);
                let port = u16::from_be_bytes([data[6], data[7]]);

                Some(SocketAddr::V4(SocketAddrV4::new(ip, port)))
            },
            6 => {
                //ipv6
                let mut ip = [0u8; 16];
                ip.copy_from_slice(&data[2..18]);
                let port = u16::from_be_bytes([data[18], data[19]]);
                let ip = Ipv6Addr::from_octets(ip);

                Some(SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0)))
            },
            _ => None,
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
