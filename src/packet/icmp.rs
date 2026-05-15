use crate::error::Error;
use std::io::Write;

pub const HEADER_SIZE: usize = 8;

pub struct IcmpV4;
pub struct IcmpV6;

pub trait Proto {
    const ECHO_REQUEST_TYPE: u8;
    const ECHO_REQUEST_CODE: u8;
    const ECHO_REPLY_TYPE: u8;
    const ECHO_REPLY_CODE: u8;
}

impl Proto for IcmpV4 {
    const ECHO_REQUEST_TYPE: u8 = 8;
    const ECHO_REQUEST_CODE: u8 = 0;
    const ECHO_REPLY_TYPE: u8 = 0;
    const ECHO_REPLY_CODE: u8 = 0;
}

impl Proto for IcmpV6 {
    const ECHO_REQUEST_TYPE: u8 = 128;
    const ECHO_REQUEST_CODE: u8 = 0;
    const ECHO_REPLY_TYPE: u8 = 129;
    const ECHO_REPLY_CODE: u8 = 0;
}

pub struct EchoRequest {
    pub ident: u16,
    pub seq_cnt: u16,
}

impl EchoRequest {
    pub fn encode<P: Proto>(&self, buffer: &mut [u8], payload: &[u8]) -> Result<Vec<u8>, Error> {
        if buffer.len() < HEADER_SIZE + payload.len() {
            return Err(Error::InvalidSize);
        }

        buffer[0] = P::ECHO_REQUEST_TYPE;
        buffer[1] = P::ECHO_REQUEST_CODE;
        buffer[2] = 0;
        buffer[3] = 0;

        buffer[4] = (self.ident >> 8) as u8;
        buffer[5] = self.ident as u8;
        buffer[6] = (self.seq_cnt >> 8) as u8;
        buffer[7] = self.seq_cnt as u8;

        if (&mut buffer[HEADER_SIZE..HEADER_SIZE + payload.len()])
            .write_all(payload)
            .is_err()
        {
            return Err(Error::InvalidSize);
        }

        write_checksum(&mut buffer[..HEADER_SIZE + payload.len()]);
        Ok(buffer.to_vec())
    }
}

pub struct EchoReply<'a> {
    pub ident: u16,
    pub seq_cnt: u16,
    pub payload: &'a [u8],
}

impl<'a> EchoReply<'a> {
    pub fn decode<P: Proto>(buffer: &'a [u8]) -> Result<Self, Error> {
        if buffer.as_ref().len() < HEADER_SIZE {
            return Err(Error::InvalidSize);
        }

        let type_ = buffer[0];
        let code = buffer[1];
        if type_ != P::ECHO_REPLY_TYPE || code != P::ECHO_REPLY_CODE {
            return Err(Error::InvalidPacket);
        }

        let ident = (u16::from(buffer[4]) << 8) + u16::from(buffer[5]);
        let seq_cnt = (u16::from(buffer[6]) << 8) + u16::from(buffer[7]);

        let payload = &buffer[HEADER_SIZE..];

        Ok(EchoReply {
            ident,
            seq_cnt,
            payload,
        })
    }
}

fn write_checksum(buffer: &mut [u8]) {
    let mut sum = 0u32;
    for word in buffer.chunks(2) {
        let mut part = u16::from(word[0]) << 8;
        if word.len() > 1 {
            part += u16::from(word[1]);
        }
        sum = sum.wrapping_add(u32::from(part));
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }

    let sum = !sum as u16;

    buffer[2] = (sum >> 8) as u8;
    buffer[3] = (sum & 0xff) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_echo_request_with_valid_checksum() {
        let request = EchoRequest {
            ident: 0x1234,
            seq_cnt: 7,
        };
        let payload = [1, 2, 3, 4];
        let mut buffer = [0; HEADER_SIZE + 4];

        let encoded = request.encode::<IcmpV4>(&mut buffer, &payload).unwrap();

        assert_eq!(encoded[0], IcmpV4::ECHO_REQUEST_TYPE);
        assert_eq!(encoded[1], IcmpV4::ECHO_REQUEST_CODE);
        assert_eq!(&encoded[4..8], &[0x12, 0x34, 0, 7]);
        assert_eq!(&encoded[8..], &payload);
        assert_eq!(checksum_sum(&encoded), 0xffff);
    }

    #[test]
    fn rejects_wrong_echo_reply_code() {
        let packet = [IcmpV4::ECHO_REPLY_TYPE, 1, 0, 0, 0, 1, 0, 1];

        assert!(matches!(
            EchoReply::decode::<IcmpV4>(&packet),
            Err(Error::InvalidPacket)
        ));
    }

    #[test]
    fn decodes_icmpv6_echo_reply() {
        let packet = [IcmpV6::ECHO_REPLY_TYPE, 0, 0, 0, 0x12, 0x34, 0, 9, 1, 2];
        let reply = EchoReply::decode::<IcmpV6>(&packet).unwrap();

        assert_eq!(reply.ident, 0x1234);
        assert_eq!(reply.seq_cnt, 9);
        assert_eq!(reply.payload, &[1, 2]);
    }

    fn checksum_sum(buffer: &[u8]) -> u16 {
        let mut sum = 0u32;
        for word in buffer.chunks(2) {
            let mut part = u16::from(word[0]) << 8;
            if word.len() > 1 {
                part += u16::from(word[1]);
            }
            sum = sum.wrapping_add(u32::from(part));
        }
        while (sum >> 16) > 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }
        sum as u16
    }
}
