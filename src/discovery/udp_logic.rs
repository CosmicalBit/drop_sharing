use std::net::{IpAddr, SocketAddr};

use crate::{
    discovery::connection::{DecodeError, Deserialize, IndicationBytes, Serialize, Size},
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
    pub fn socket(&self) -> SocketAddr {
        self.socket
    }
    pub fn figer_print(&self) -> blake3::Hash {
        self.finger_print
    }
    pub fn set_ip(&mut self, ip: IpAddr) {
        self.socket.set_ip(ip);
    }
}

impl Serialize for DiscoveryMessage {
    fn serialize(&self) -> Box<[u8]> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::MagicInit as u8);
        vec.extend_from_slice(&self.socket.serialize());
        vec.extend_from_slice(self.finger_print.as_bytes());

        vec.into_boxed_slice()
    }
}

impl Deserialize for DiscoveryMessage {
    type Output = Self;

    const SIZE: Size = Size::Fixed(DISCOVERY_MESSAGE_SIZE);

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < DISCOVERY_MESSAGE_SIZE {
            return Err(DecodeError::Truncated {
                expected: DISCOVERY_MESSAGE_SIZE,
                actual: data.len(),
            });
        }
        let first = data[0];
        if first != IndicationBytes::MagicInit as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::MagicInit,
                actual: first,
            });
        }
        let socket_indication = data[1];
        if socket_indication != IndicationBytes::MagicInit as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::MagicInit,
                actual: socket_indication,
            });
        }
        if !matches!(data.get(2), Some(4 | 6)) {
            return Err(DecodeError::InvalidValue("unsupported IP version"));
        }

        let socket = SocketAddr::deserialize(&data[1..1 + SOCKET_ADDR_SIZE])?;
        let fingerprint_bytes: &[u8; FINGERPRINT_SIZE] = data[1 + SOCKET_ADDR_SIZE..DISCOVERY_MESSAGE_SIZE]
            .try_into()
            .expect("fingerprint slice is exactly 32 bytes");
        let finger_print = blake3::Hash::from_bytes(*fingerprint_bytes);

        Ok(Self { socket, finger_print })
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

        assert!(DiscoveryMessage::deserialize(&serialized[..serialized.len() - 1]).is_err());

        serialized[1] = 0;
        assert!(DiscoveryMessage::deserialize(&serialized).is_err());
    }
}
