use std::net::SocketAddr;

use crate::{
    discovery::connection::{Deserialize, IndicationBytes, Serialize, Size},
    identity::identity_definition::Identification,
};

const SOCKET_ADDR_SIZE: usize = 20;
const FINGERPRINT_SIZE: usize = 32;
const DISCOVERY_MESSAGE_SIZE: usize = 1 + SOCKET_ADDR_SIZE + FINGERPRINT_SIZE;

pub struct DiscoveryMessage {
    socket: SocketAddr,
    finger_print: blake3::Hash,
}



impl DiscoveryMessage {
    pub fn new(addr: SocketAddr, identification: &Identification) -> Self {
        DiscoveryMessage {
            socket: addr,
            finger_print: identification.hash(),
        }
    }
    pub fn socket(&self)->SocketAddr{
        self.socket
    }
    pub fn figer_print(&self)-> blake3::Hash{
        self.finger_print
    }
}

impl Serialize for DiscoveryMessage {
    fn serialize(&self) -> Vec<u8> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::MagicInit as u8);
        vec.extend_from_slice(&self.socket.serialize());
        vec.extend_from_slice(self.finger_print.as_bytes());

        vec
    }
}

impl Deserialize for DiscoveryMessage {
    type Output = Self;

    const SIZE: Size = Size::Dynamic {
        header_size: 3,
        total_size: |header| {
            if header.first().copied()? != IndicationBytes::MagicInit as u8
                || header.get(1).copied()? != IndicationBytes::MagicInit as u8
                || !matches!(header.get(2), Some(4 | 6))
            {
                return None;
            }

            Some(DISCOVERY_MESSAGE_SIZE)
        },
    };

    fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() != DISCOVERY_MESSAGE_SIZE
            || data.first().copied()? != IndicationBytes::MagicInit as u8
            || data.get(1).copied()? != IndicationBytes::MagicInit as u8
            || !matches!(data.get(2), Some(4 | 6))
        {
            return None;
        }

        let socket = SocketAddr::deserialize(data.get(1..1 + SOCKET_ADDR_SIZE)?)?;
        let finger_print = blake3::Hash::from_bytes(data.get(1 + SOCKET_ADDR_SIZE..)?.try_into().ok()?);

        Some(Self { socket, finger_print })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(socket: SocketAddr) {
        let message = DiscoveryMessage {
            socket,
            finger_print: blake3::hash(b"test identity"),
        };

        let serialized = message.serialize();
        let deserialized = DiscoveryMessage::deserialize(&serialized).unwrap();

        assert_eq!(deserialized.socket, message.socket);
        assert_eq!(deserialized.finger_print, message.finger_print);
    }

    #[test]
    fn discovery_message_ipv4_round_trip() {
        round_trip("127.0.0.1:4242".parse().unwrap());
    }

    #[test]
    fn discovery_message_ipv6_round_trip() {
        round_trip("[::1]:4242".parse().unwrap());
    }

    #[test]
    fn discovery_message_rejects_invalid_data() {
        let message = DiscoveryMessage {
            socket: "127.0.0.1:4242".parse().unwrap(),
            finger_print: blake3::hash(b"test identity"),
        };
        let mut serialized = message.serialize();

        assert!(DiscoveryMessage::deserialize(&serialized[..serialized.len() - 1]).is_none());

        serialized[1] = 0;
        assert!(DiscoveryMessage::deserialize(&serialized).is_none());
    }
}
