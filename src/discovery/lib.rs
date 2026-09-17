use std::{
    io::{self},
    net::SocketAddr,
};

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
    pub async fn send(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.mode.stream.write_all(bytes).await?;

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
    fn serialize(self) -> Vec<u8>;
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

    fn deserialize(data: &[u8]) -> Option<Self>;
}

pub trait Recieve {
    async fn recieve(&mut self, buffer: &mut [u8]) -> io::Result<()>;
}
