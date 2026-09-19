use std::{
    io::{self},
    net::SocketAddr,
};

use ml_dsa::SignatureEncoding;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::identity::identity::IdentityContext;

#[repr(u8)]
pub enum IndicationBytes {
    MagicInit = 1,
    HostInfo = 2,
    TransferResponse = 3,
    HostName = 4,
    PubKeySend = 5,
    Ciphertxt = 6,
    PublicIdentKey,
}

impl From<u8> for IndicationBytes {
    fn from(value: u8) -> Self {
        match value {
            0x01 => Self::MagicInit,
            _ => todo!(),
        }
    }
}

pub struct Tcp {
    stream: TcpStream,
}
impl Tcp {
    pub fn new(stream: TcpStream) -> Self {
        Self { stream }
    }
}
pub struct Udp {
    stream: UdpSocket,
}
impl Udp {
    pub fn new(stream: UdpSocket) -> Self {
        Self { stream }
    }
}

pub struct Connection<Mode> {
    mode: Mode,
}

impl Connection<Tcp> {
    pub async fn new_listen() -> io::Result<(Self, SocketAddr)> {
        let listener = TcpListener::bind("0.0.0.0:0").await?;

        let my_addr = listener.local_addr()?;

        let (stream, _) = listener.accept().await?;

        let tcp = Tcp::new(stream);
        let connection = Connection { mode: tcp };

        Ok((connection, my_addr))
    }
    //it takes old connection just to make sure i dont accidentaly use it
    pub async fn new_send_n_listen(addr: SocketAddr, _old_connection: Connection<Udp>) -> io::Result<Self> {
        let socket = TcpStream::connect(addr).await?;

        Ok(Self { mode: Tcp::new(socket) })
    }
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let socket = TcpListener::bind(addr).await?;

        let (stream, _sender) = socket.accept().await?;

        Ok(Self { mode: Tcp::new(stream) })
    }
}

impl Send for Connection<Tcp> {
    async fn send(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.mode.stream.write_all(buffer).await?;

        Ok(())
    }
}
impl SendSign for Connection<Tcp> {
    async fn send_n_sign(&mut self, buffer: &[u8], identity_context: &IdentityContext) -> io::Result<()> {
        let signature = identity_context.sign(buffer).to_bytes();
        let mut signed = Vec::with_capacity(buffer.len() + signature.len());
        signed.extend_from_slice(buffer);
        signed.extend_from_slice(&signature);
        self.mode.stream.write_all(&signed).await?;

        Ok(())
    }
}

impl Recieve for Connection<Tcp> {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        self.mode.stream.readable().await?;

        self.mode.stream.read_exact(buffer).await?;

        Ok(())
    }
}

impl Connection<Udp> {
    pub async fn new_listen() -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:4242").await?;

        Ok(Self { mode: Udp::new(socket) })
    }

    async fn new_broadcast() -> io::Result<Self> {
        let udp = UdpSocket::bind("0.0.0.0:0").await?;

        udp.set_broadcast(true)?;

        Ok(Self { mode: Udp::new(udp) })
    }
    pub async fn start_broadcast_and_send(data: &[u8]) -> io::Result<()> {
        let broadcast = Connection::<Udp>::new_broadcast().await?;

        broadcast.mode.stream.writable().await?;

        broadcast.mode.stream.send_to(data, "255.255.255.255:4242").await?;

        Ok(())
    }
}

impl Recieve for Connection<Udp> {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        self.mode.stream.readable().await?;

        self.mode.stream.recv_from(buffer).await?;

        Ok(())
    }
}
pub trait Serialize {
    fn serialize(&self) -> Vec<u8>;
}

pub enum Size {
    Fixed(usize),
    Dynamic {
        header_size: usize,
        total_size: fn(&[u8]) -> Option<usize>,
    },
}
pub trait Deserialize: Sized {
    const SIZE: Size;
    type Output;
    fn deserialize(data: &[u8]) -> Option<Self::Output>;
}

pub trait Recieve {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()>;
}
pub trait Send {
    async fn send(&mut self, buffer: &[u8]) -> io::Result<()>;
}
pub trait SendSign {
    async fn send_n_sign(&mut self, buffer: &[u8], identity_context: &IdentityContext) -> io::Result<()>;
}
