use crate::start_handler::FData;
use crate::start_handler::walk;
use std::{collections::hash_map::Keys, ffi::OsStr, marker::PhantomData, net::Ipv4Addr, os::unix::ffi::OsStrExt, path::Path, u8};
use tokio::{
    io,
    io::AsyncReadExt,
    net::{TcpListener, TcpStream, UdpSocket},
};

use crate::start_handler::FileInfo;

const APP_IDENT: &[u8] = b"DropSharing";

pub enum Protocol<'device_name> {
    Poor(DiscoveryData<'device_name, Poor>),
    Ritch(DiscoveryData<'device_name, Ritch>),
}

pub trait Pack {
    fn compact(&self) -> Vec<u8>;
}

impl Protocol<'_> {
    fn kind_byte(&self) -> u8 {
        match self {
            Self::Poor(_) => IndicatioByte::Poor as u8,
            Self::Ritch(_) => IndicatioByte::Rich as u8,
        }
    }
}

#[repr(u8)]
#[derive(Clone)]
enum IndicatioByte {
    Poor = 1,
    Rich = 2,
    Ok = 3,
    RequestRich = 4,
}

impl From<u8> for IndicatioByte {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Poor,
            2 => Self::Rich,
            3 => Self::Ok,
            4 => Self::RequestRich,
            _ => panic!(),
        }
    }
}

impl Pack for Protocol<'_> {
    //thsi is sending a
    //compacts it , the [`APP_IDENT`] comes first
    fn compact(&self) -> Vec<u8> {
        let mut vec: Vec<u8> = Vec::new();
        let byte = self.kind_byte();

        match self {
            Protocol::Poor(data) => {
                vec.extend_from_slice(APP_IDENT);
                vec.extend_from_slice(&[byte]);
                vec.extend_from_slice(&data.device_name_len);
                vec.extend_from_slice(data.device_name);
                //dont send [`State`] bcs its poor meaning that it carries no extra info

                vec
            },

            Protocol::Ritch(data) => {
                //TODO: implement the rich option, needs to pack rich and for that needs to pack FileInfo and
                // potentially change the struct im stupid i didnt remember thsi commend only matters
                // for the ritch path
                todo!("implement the rich option ");
                vec.extend_from_slice(APP_IDENT);
                vec.extend_from_slice(&[byte]);
                vec.extend_from_slice(&data.device_name_len);
                vec.extend_from_slice(data.device_name);
                vec.extend_from_slice(&data.state.data_size);
                vec.extend_from_slice(&data.state.data.as_slice());

                vec
            },
        }
    }
}

struct Poor;
struct Ritch {
    data_size: [u8; 4], //u32
    data: Vec<FileInfo>,
}

struct DiscoveryData<'device_name, State> {
    pub app_ident: &'static [u8],
    pub device_name_len: [u8; 2],
    pub device_name: &'device_name [u8],
    pub state: State,
}

impl<'a> DiscoveryData<'a, Poor> {
    fn new(device_name: &'a OsStr) -> Self {
        let len = device_name.len() as u16;
        let len: [u8; 2] = len.to_be_bytes();

        DiscoveryData::<Poor> {
            app_ident: APP_IDENT,
            device_name_len: len,
            device_name: device_name.as_bytes(),
            state: Poor,
        }
    }
}

const DISCOVERY_PORT: u16 = 4242;

pub async fn first_time_poor_discover(socket: UdpSocket) -> io::Result<()> {
    socket.set_broadcast(true)?;

    //first ask if user wants with poor packet, no [`FileInfo`]
    let host = hostname::get()?;
    let discovery_data = Protocol::Poor(DiscoveryData::new(&host));
    let compacted = discovery_data.compact();

    socket.send_to(&compacted, ("255.255.255.255:", DISCOVERY_PORT)).await?;

    Ok(())
}

///reads and returnst the indication byte
pub async fn read_recieving_byte_identifier(stream: &mut TcpStream) -> io::Result<IndicatioByte> {
    let mut byte = [0u8; 1];

    stream.read_exact(&mut byte).await?;

    let byte = IndicatioByte::from(byte[0]);

    Ok(byte)
}

///reads the ipv4 addr
async fn read_ipv4_addr(stream: &mut TcpStream) -> io::Result<Ipv4Addr> {
    //ipv4 addreas is 2 bytes
    let mut address = [0u8; 4];

    stream.read_exact(&mut address).await?;

    Ok(Ipv4Addr::from_octets(address))
}

///hadles any possible answer from sender
pub async fn handle_answer(socket: TcpListener, dir: &Path) -> io::Result<()> {
    let (mut stream, addr) = socket.accept().await?;

    match read_recieving_byte_identifier(&mut stream) {
        IndicatioByte::Ok => {
            // send data btw answer should contain a port
            let addr = read_ipv4_addr(&mut stream).await?;
            //walk for data
           let collection_of_fdata =  walk::<FData>(dir)?;


           
        },

        IndicatioByte::RequestRich => todo!(),
    }

    Ok(())
}

