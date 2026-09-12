use crate::start_handler::FileInfo;
use std::{collections::hash_map::Keys, ffi::OsStr, marker::PhantomData, os::unix::ffi::OsStrExt};
use tokio::{io, net::UdpSocket};

const APP_IDENT: &[u8] = b"DropSharing";

pub enum Protocol<'device_name> {
    Poor(DiscoveryData<'device_name, Poor>),
    Ritch(DiscoveryData<'device_name, Ritch>),
}

impl Protocol<'_> {
    fn kind_byte(&self) -> u8 {
        match self {
            Self::Poor(_) => 1,
            Self::Ritch(_) => 2,
        }
    }

    //compacts it , the [`APP_IDENT`] comes first
     fn compact(&self) {
        let vec: Vec<u8> = Vec::new();
        let byte = self.kind_byte();

        todo!();
        match self {
            
            Protocol::Poor(data) => {
                vec.extend_from_slice(APP_IDENT);
                vec.extend_from_slice(&byte);
                
                
            },
        }
    }
}

struct Poor;
struct Ritch {
    data_size: u32,
    data: Vec<FileInfo>,
}
struct DiscoveryData<'device_name, State> {
    app_ident: &'static [u8],
    device_name_len: [u8; 2],
    device_name: &'device_name [u8],
    state: State,
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

async fn first_time_poor_discover() -> io::Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;

    socket.set_broadcast(true)?;

    //first ask if user wants with poor packet, no [`FileInfo`]
    let host = hostname::get()?;
    let discovery_data = Protocol::Poor(DiscoveryData::new(&host));

    socket.send_to(buf, addr);

    Ok(())
}
