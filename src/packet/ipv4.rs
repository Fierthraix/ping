use std::net::{IpAddr, Ipv4Addr};

use crate::error::Error;

const MINIMUM_PACKET_SIZE: usize = 20;

#[derive(Debug, PartialEq)]
pub enum IpV4Protocol {
    Icmp,
}

impl IpV4Protocol {
    fn decode(data: u8) -> Option<Self> {
        match data {
            1 => Some(IpV4Protocol::Icmp),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct IpV4Packet<'a> {
    pub protocol: IpV4Protocol,
    pub source: IpAddr,
    pub ttl: u8,
    pub data: &'a [u8],
}

impl<'a> IpV4Packet<'a> {
    pub fn decode(data: &'a [u8]) -> Result<Self, Error> {
        if data.len() < MINIMUM_PACKET_SIZE {
            return Err(Error::TooSmallHeader);
        }
        let byte0 = data[0];
        let version = (byte0 & 0xf0) >> 4;
        let header_size = 4 * ((byte0 & 0x0f) as usize);

        if version != 4 {
            return Err(Error::InvalidVersion);
        }

        if header_size < MINIMUM_PACKET_SIZE {
            return Err(Error::InvalidHeaderSize);
        }

        if data.len() < header_size {
            return Err(Error::InvalidHeaderSize);
        }

        let protocol = match IpV4Protocol::decode(data[9]) {
            Some(protocol) => protocol,
            None => return Err(Error::UnknownProtocol),
        };

        Ok(Self {
            protocol,
            source: IpAddr::V4(Ipv4Addr::new(data[12], data[13], data[14], data[15])),
            ttl: data[8],
            data: &data[header_size..],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_ipv4_header_metadata_and_payload() {
        let packet = [
            0x45, 0, 0, 28, 0, 0, 0, 0, 63, 1, 0, 0, 192, 0, 2, 1, 8, 8, 8, 8, 0, 0, 0, 0, 0, 1, 0,
            2,
        ];

        let decoded = IpV4Packet::decode(&packet).unwrap();

        assert_eq!(decoded.protocol, IpV4Protocol::Icmp);
        assert_eq!(decoded.source, IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)));
        assert_eq!(decoded.ttl, 63);
        assert_eq!(decoded.data, &[0, 0, 0, 0, 0, 1, 0, 2]);
    }

    #[test]
    fn rejects_too_small_ihl() {
        let packet = [
            0x44, 0, 0, 20, 0, 0, 0, 0, 63, 1, 0, 0, 192, 0, 2, 1, 8, 8, 8, 8,
        ];

        assert!(matches!(
            IpV4Packet::decode(&packet),
            Err(Error::InvalidHeaderSize)
        ));
    }
}
