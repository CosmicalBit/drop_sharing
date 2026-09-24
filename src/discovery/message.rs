//! this module provides mainly [`Message`] to have an abstraction to recieving anny data type over network
//!
//! it also defines [`TransferDesision`] and [`TransferResponse`] for network communication and explicitnessa acception
use std::io;

use strum::EnumIter;

use crate::{
    discovery::{
        connection::{
            DecodeError, Deserialize,
            IndicationBytes::{self},
            Recieve, Serialize, Size,
        }, hostinfo::Host,
    }, identity::identity_definition::{IdentityContext, SIGNATURE_ADDED_SIZE},
};

///[`Message`] is used to be a generic stateless helper for recieving data over the network
#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Message;

/// [`TransferDesision`] is used to represent if a transfier was accepted or no
#[repr(u8)]
#[derive(PartialEq, Eq, Clone, Debug, EnumIter)]
pub enum TransferDesision {
    Accepted = 1,
    Rejected = 0,
}

impl Serialize for TransferDesision {
    fn serialize(&self) -> Box<[u8]> {
        vec![self.clone() as u8].into_boxed_slice()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct TransferResponse {
    host: Host,
    decision: TransferDesision,
}

impl TransferResponse {
    pub fn accepted(&self) -> bool {
        self.decision == TransferDesision::Accepted
    }
}

impl TransferResponse {
    pub fn new(decision: TransferDesision) -> io::Result<Self> {
        let host = Host::new()?;

        Ok(Self { host, decision })
    }
}

impl Serialize for TransferResponse {
    fn serialize(&self) -> Box<[u8]> {
        let mut vec = Vec::new();

        vec.push(IndicationBytes::TransferResponse as u8);
        vec.extend_from_slice(&self.host.serialize());
        vec.extend_from_slice(&self.decision.serialize());

        vec.into_boxed_slice()
    }
}
impl Deserialize for TransferDesision {
    const SIZE: Size = Size::Fixed(1);
    type Output = Self;

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        const ACCEPT: u8 = TransferDesision::Accepted as u8;
        const REJECT: u8 = TransferDesision::Rejected as u8;

        match *data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })? {
            ACCEPT => Ok(TransferDesision::Accepted),
            REJECT => Ok(TransferDesision::Rejected),
            _ => Err(DecodeError::InvalidValue("invalid transfer decision")),
        }
    }
}

impl Deserialize for TransferResponse {
    type Output = Self;
    const SIZE: Size = Size::Dynamic {
        header_size: 6,
        total_size: |header| {
            let indication = *header.first().ok_or(DecodeError::Truncated {
                expected: 1,
                actual: header.len(),
            })?;
            if indication != IndicationBytes::TransferResponse as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::TransferResponse,
                    actual: indication,
                });
            }
            let host_indication = *header.get(1).ok_or(DecodeError::Truncated {
                expected: 2,
                actual: header.len(),
            })?;
            if host_indication != IndicationBytes::HostName as u8 {
                return Err(DecodeError::UnexpectedIndicationType {
                    expected: IndicationBytes::HostName,
                    actual: host_indication,
                });
            }
            let len_bytes = header.get(2..6).ok_or(DecodeError::Truncated {
                expected: 6,
                actual: header.len(),
            })?;
            let host_len = u32::from_be_bytes(len_bytes.try_into().expect("2..6 is exactly 4 bytes")) as usize;
            Ok(7 + host_len)
        },
    };

    fn deserialize(data: &[u8]) -> Result<Self, DecodeError> {
        let indication = *data.first().ok_or(DecodeError::Truncated {
            expected: 1,
            actual: data.len(),
        })?;
        if indication != IndicationBytes::TransferResponse as u8 {
            return Err(DecodeError::UnexpectedIndicationType {
                expected: IndicationBytes::TransferResponse,
                actual: indication,
            });
        }
        let len_bytes = data.get(2..6).ok_or(DecodeError::Truncated {
            expected: 6,
            actual: data.len(),
        })?;
        let host_len = u32::from_be_bytes(len_bytes.try_into().expect("2..6 is exactly 4 bytes")) as usize;
        let host_end = 6 + host_len;
        let host_bytes = data.get(1..host_end).ok_or(DecodeError::InvalidLen {
            declared: host_len,
            available: data.len().saturating_sub(6),
        })?;
        let host = Host::deserialize(host_bytes)?;
        let decision_bytes = data.get(host_end..host_end + 1).ok_or(DecodeError::Truncated {
            expected: host_end + 1,
            actual: data.len(),
        })?;
        let decision = TransferDesision::deserialize(decision_bytes)?;

        Ok(Self { host, decision })
    }
}

impl Message {
    pub async fn receive<T: Deserialize>(connection: &mut impl Recieve) -> io::Result<T::Output> {
        const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let total = total_size(&data).map_err(invalid_data)?;
                if total > MAX_MESSAGE_SIZE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "message is too large"));
                }

                data.resize(total, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        T::deserialize(&data).map_err(invalid_data)
    }

    pub async fn receive_signed<T: Deserialize>(
        connection: &mut impl Recieve,
        identity_context: &IdentityContext,
    ) -> io::Result<T::Output> {
        const MAX_MESSAGE_SIZE: usize = 1024 * 1024;

        let data = match T::SIZE {
            Size::Fixed(size) => {
                let mut data = vec![0; size + SIGNATURE_ADDED_SIZE];
                connection.recieve(&mut data).await?;
                data
            },

            Size::Dynamic { header_size, total_size } => {
                let mut data = vec![0; header_size];
                connection.recieve(&mut data).await?;

                let total = total_size(&data).map_err(invalid_data)?;
                if total > MAX_MESSAGE_SIZE {
                    return Err(io::Error::new(io::ErrorKind::InvalidData, "message is too large"));
                }

                data.resize(total + SIGNATURE_ADDED_SIZE, 0);
                connection.recieve(&mut data[header_size..]).await?;

                data
            },
        };

        let data = identity_context.check_signature(&data)?;

        T::deserialize(data).map_err(invalid_data)
    }
}

fn invalid_data(error: DecodeError) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, error)
}

#[cfg(test)]
mod test {
    use strum::IntoEnumIterator;

    use super::*;

    #[test]
    fn round_trip_of_transfer_responce() {
        for decision in TransferDesision::iter() {
            let responce = TransferResponse::new(decision).unwrap();

            let seri = responce.clone().serialize();

            let deri = TransferResponse::deserialize(&seri).unwrap();

            assert_eq!(deri, responce);
        }
    }
}
