use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};

use crate::discovery::{
    hostinfo::{Host, HostInfo},
    lib::{
        Connection, Deserialize,
        IndicationBytes::{self},
        Recieve, Send, Serialize, Size, Tcp,
    },
};

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Message {
    Address(SocketAddr),
    HostName(HostInfo),
    TransferResponse(TransferResponse),
}

#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Debug)]
pub enum TransferDesision {
    Accepted = 1,
    Rejected = 0,
}
impl Serialize for TransferDesision {
    fn serialize(self) -> Vec<u8> {
        vec![self as u8]
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TransferResponse {
    host: Host,
    decision: TransferDesision,
}

impl TransferResponse {
    pub fn new(decision: TransferDesision) -> io::Result<Self> {
        let host = Host::new()?;

        Ok(Self { host, decision })
    }
}

impl Serialize for TransferResponse {
    fn serialize(self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::TransferResponse as u8);
        vec.extend_from_slice(&self.host.serialize());
        vec.extend_from_slice(&self.decision.serialize());
        vec
    }
}

impl Message {
    pub async fn send(self, connection: &mut Connection<Tcp>) -> io::Result<()> {
        match self {
            Message::Address(addr) => connection.send(&addr.serialize()).await,
            Message::HostName(name) => connection.send(&name.serialize()).await,
            Message::TransferResponse(response) => connection.send(&response.serialize()).await,
        }
    }
    pub async fn receive<T: Deserialize>(connection: &mut impl Recieve) -> io::Result<Option<T>> {
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
}

impl Serialize for SocketAddr {
    fn serialize(self) -> Vec<u8> {
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
impl Serialize for Message {
    fn serialize(self) -> Vec<u8> {
        match self {
            Self::Address(addr) => addr.serialize(),
            Self::HostName(host) => host.serialize(),
            Self::TransferResponse(response) => response.serialize(),
        }
    }
}

impl Deserialize for SocketAddr {
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
    use super::*;

    fn msg_sckaddr() -> (Message, Message) {
        let ipv4: SocketAddr = "127.0.0.1:4242".parse().unwrap();
        let ipv6: SocketAddr = "[::1]:4242".parse().unwrap();

        (Message::Address(ipv4), Message::Address(ipv6))
    }

    #[test]
    fn check_deserialized_serialize_round_trip() {
        let (org_ipv4, ipv6) = msg_sckaddr();

        let serialized = org_ipv4.clone().serialize();

        let deserialized = SocketAddr::deserialize(&serialized).unwrap();

        assert_eq!(Message::Address(deserialized), org_ipv4);

        let serialized = ipv6.clone().serialize();

        let deserialized = SocketAddr::deserialize(&serialized).unwrap();

        assert_eq!(Message::Address(deserialized), ipv6);
    }
}
