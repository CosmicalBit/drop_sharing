use std::{
    io::{self, Error},
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use hostname::get;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

#[repr(u8)]
pub enum IndicationBytes {
    MagicInit = 0x1,
    HostName = 0x2,
}

impl From<u8> for IndicationBytes {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::MagicInit,
            _ => todo!(),
        }
    }
}

pub enum Message {
    Address(SocketAddr),
    HostName(HostName),
}

impl Serialize for Message {
    fn serialize(self) -> Vec<u8> {
        match self {
            Self::Address(addr) => {
                match addr {
                    SocketAddr::V4(addr) => {
                        let mut out = Vec::with_capacity(9);

                        out.push(IndicationBytes::MagicInit as u8);
                        out.push(4); // ipv4
                        out.extend_from_slice(&addr.ip().to_string().as_bytes());
                        out.extend_from_slice(&addr.port().to_be_bytes());
                        return out;
                    },
                    SocketAddr::V6(addr) => {
                        let mut out = Vec::with_capacity(20);

                        out.push(IndicationBytes::MagicInit as u8);
                        out.push(6); // ipv6
                        out.extend_from_slice(&addr.ip().to_string().as_bytes());
                        out.extend_from_slice(&addr.port().to_be_bytes());
                        return out;
                    },
                }
                todo!()
            },
        }
    }
}

pub struct HostName {
    len: u16,
    hostname: String,
}

impl HostName {
    pub fn new_connect() -> io::Result<Self> {
        let name = hostname::get()?
            .to_str()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "failed conversion"))?
            .to_string();

        Ok(Self {
            len: name.len() as u16,
            hostname: name,
        })
    }
}

pub struct Connection {
    pub stream: TcpStream,
}

impl Connection {
    pub async fn new_connect(addr: SocketAddrV4) -> io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;

        Ok(Self { stream })
    }
    pub async fn new_listen() -> io::Result<(Self, SocketAddr)> {
        let listener = TcpListener::bind("0.0.0.0:0").await?;

        let my_addr = listener.local_addr()?;

        let (stream, _) = listener.accept().await?;

        Ok((Self { stream }, my_addr))
    }
    pub async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.stream.write_all(&bytes).await?;

        Ok(())
    }
}

pub trait Serialize {
    fn serialize(self) -> Vec<u8>;
}
pub trait Deserialize {
    fn deserialize(self) -> Vec<u8>;
}
