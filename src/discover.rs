use crate::start_handler::FileInfo;
use std::{collections::hash_map::Keys, ffi::OsStr, marker::PhantomData, os::unix::ffi::OsStrExt};
use tokio::{io, net::UdpSocket};

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
            Self::Poor(_) => 1,
            Self::Ritch(_) => 2,
        }
    }
}

impl Pack for Protocol<'_> {
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
                //TODO: implement the rich option, needs to pack rich and for that needs to pack FileInfo and potentially change the struct
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

async fn first_time_poor_discover() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    socket.set_broadcast(true)?;

    //first ask if user wants with poor packet, no [`FileInfo`]
    let host = hostname::get()?;
    let discovery_data = Protocol::Poor(DiscoveryData::new(&host));
    let compacted = discovery_data.compact();
    
    socket.send_to(&compacted, ("255.255.255.255:",DISCOVERY_PORT)).await?;

    
    Ok(())
}
