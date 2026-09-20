//! [`Connection`] defines a connection typses and struct connection.
//! Its all network + generalize protocol definitions, sutch as indication bytes.
//! Everything thing that is consistent over the protocol and applies over or is related too [`Connection`]

use std::{
    io::{self},
    net::SocketAddr,
};

use ml_dsa::SignatureEncoding;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::identity::identity_definition::IdentityContext;

///Indication bytes is is used in tcp and udp connections to identify what we are reading
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicationBytes {
    MagicInit = 1,
    HostInfo = 2,
    TransferResponse = 3,
    HostName = 4,
    PubKeySend = 5,
    Ciphertxt = 6,
    PublicIdentKey = 7,
    File = 8,
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

///[`Connection<Mode>`] is used to send data and do data operations
/// over [`Tcp`] or [`Udp`]
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
    fn serialize(&self) -> Box<[u8]>;
}

pub enum Size {
    Fixed(usize),
    Dynamic {
        header_size: usize,
        total_size: fn(&[u8]) -> Result<usize, DecodeError>,
    },
}

#[derive(Debug)]
pub enum DecodeError {
    Truncated { expected: usize, actual: usize },
    UnexpectedIndicationType { expected: IndicationBytes, actual: u8 },
    InvalidUtf8(std::str::Utf8Error),
    InvalidLen { declared: usize, available: usize },
    InvalidValue(&'static str),
}
impl From<std::str::Utf8Error> for DecodeError {
    fn from(value: std::str::Utf8Error) -> Self {
        DecodeError::InvalidUtf8(value)
    }
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { expected, actual } => {
                write!(formatter, "truncated message: expected at least {expected} bytes, got {actual}")
            },
            Self::UnexpectedIndicationType { expected, actual } => {
                write!(formatter, "unexpected indication byte: expected {}, got {actual}", *expected as u8)
            },
            Self::InvalidUtf8(error) => error.fmt(formatter),
            Self::InvalidLen { declared, available } => {
                write!(
                    formatter,
                    "invalid declared length {declared}; only {available} bytes are available"
                )
            },
            Self::InvalidValue(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for DecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            _ => None,
        }
    }
}

///[`Deserialize`] is a trait used by everything that can be sent over the network
/// it converts the wanted data to bytes
/// it specifies [`Size`] which specifies if the data size is fixed or no
///
/// in fixed messages [`Size::Fixed`] contains the total size of the message including [`IndicationBytes`]
///
/// in case that its not fixed you use the [`Size::Dynamic`] that contains the header size
/// and a fucntion that determines the full message size from tat header
///
/// [`Output`](Deserialize::Output) is the type returned gy deserialization, it normally is `Selfl` but sometimes it may
/// be different bcs we might not want to constuct the original object. Ex: it might be unsafe bcs the whole object was
/// not sendt over network, for exemple private keys arent sent
pub trait Deserialize: Sized {
    const SIZE: Size;
    type Output;
    fn deserialize(data: &[u8]) -> Result<Self::Output, DecodeError>;
}

///the trait [`Recieve`] just specifies the recieve func signature to make make consistent over [`Connection<Mode>`]
/// that can be [`Tcp`] or [`Udp`]
pub trait Recieve {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()>;
}

pub trait Send {
    async fn send(&mut self, buffer: &[u8]) -> io::Result<()>;
}
pub trait SendSign {
    async fn send_n_sign(&mut self, buffer: &[u8], identity_context: &IdentityContext) -> io::Result<()>;
}
