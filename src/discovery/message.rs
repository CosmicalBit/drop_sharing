use std::{
    io::{self, Error},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6},
};
use hostname::get;
use crate::discovery::lib::{
    Connection, Deserialize,
    IndicationBytes::{self, MagicInit},
    Serialize, Udp,
};

pub enum Message {
    Address(SocketAddr),
    HostName(HostInfo),
}

impl Serialize for Message {
    fn serialize(self) -> Vec<u8> {
        match self {
            Self::Address(addr) => {
                match addr {
                    SocketAddr::V4(addr) => {
                        let mut out = Vec::with_capacity(20);

                        out.push(IndicationBytes::MagicInit as u8);
                        out.push(4); // ipv4
                        out.extend_from_slice(&addr.ip().octets());
                        out.extend_from_slice(&addr.port().to_be_bytes());
                        out.extend_from_slice(&[0u8; 12]);

                        return out;
                    },
                    SocketAddr::V6(addr) => {
                        let mut out = Vec::with_capacity(20);

                        out.push(IndicationBytes::MagicInit as u8);
                        out.push(6); // ipv6
                        out.extend_from_slice(&addr.ip().octets());
                        out.extend_from_slice(&addr.port().to_be_bytes());
                        return out;
                    },
                }
                todo!()
            },
        }
    }
}

impl Message {
    pub fn deserialize_socket_addr( connection: &Connection<Udp>) -> Option<SocketAddr> {
        let mut addr = [0u8; 20];
        connection.read_exact(&mut addr);

        SocketAddr::deserialize(&addr)
    }
}
impl Deserialize for SocketAddr {
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
            _ => return None,
        }
    }
}
