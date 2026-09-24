//! [`Connection`] defines a connection typses and struct connection.
//! Its all network + generalize protocol definitions, sutch as indication bytes.
//! Everything thing that is consistent over the protocol and applies over or is related too [`Connection`]

use std::{io, net::Ipv4Addr};

use ml_dsa::SignatureEncoding;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::identity::identity_definition::IdentityContext;

const LISTENER_PORT: u16 = 50000;

///Indication bytes identify messages sent over the connection.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndicationBytes {
    HostInfo = 2,
    TransferResponse = 3,
    HostName = 4,
    PubKeySend = 5,
    Ciphertxt = 6,
    PublicIdentKey = 7,
    File = 8,
}

/// A TCP stream over the Wi-Fi Direct group.
pub struct Connection {
    stream: TcpStream,
}

impl Connection {
    pub async fn new_listen() -> io::Result<Self> {
        let listener = TcpListener::bind(("0.0.0.0", LISTENER_PORT)).await?;
        let (stream, _) = listener.accept().await?;

        Ok(Self { stream })
    }
    pub async fn connect(ip: Ipv4Addr) -> io::Result<Self> {
        let stream = TcpStream::connect((ip, LISTENER_PORT)).await?;

        Ok(Self { stream })
    }
}

impl Send for Connection {
    async fn send(&mut self, buffer: &[u8]) -> io::Result<()> {
        self.stream.write_all(buffer).await?;

        Ok(())
    }
}
impl SendSign for Connection {
    async fn send_n_sign(&mut self, buffer: &[u8], identity_context: &IdentityContext) -> io::Result<()> {
        let signature = identity_context.sign(buffer).to_bytes();
        let mut signed = Vec::with_capacity(buffer.len() + signature.len());
        signed.extend_from_slice(buffer);
        signed.extend_from_slice(&signature);
        self.stream.write_all(&signed).await?;

        Ok(())
    }
}

impl Recieve for Connection {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        self.stream.readable().await?;

        self.stream.read_exact(buffer).await?;

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

/// The receive operation used by the message decoder.
pub trait Recieve {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()>;
}

pub trait Send {
    async fn send(&mut self, buffer: &[u8]) -> io::Result<()>;
}
pub trait SendSign {
    async fn send_n_sign(&mut self, buffer: &[u8], identity_context: &IdentityContext) -> io::Result<()>;
}
