use std::net::IpAddr;

use crate::error::Result;
use crate::packet::{
    EchoReply as PacketEchoReply, EchoRequest as PacketEchoRequest, IcmpV4, IcmpV6, IpV4Packet,
};

#[derive(Debug)]
pub struct EchoRequest {
    pub destination: IpAddr,
    pub ident: u16,
    pub seq_cnt: u16,
}

impl EchoRequest {
    pub fn new(destination: IpAddr, ident: u16, seq_cnt: u16) -> Self {
        EchoRequest {
            destination,
            ident,
            seq_cnt,
        }
    }

    pub(crate) fn encode_with_payload(&self, payload: &[u8]) -> Result<Vec<u8>> {
        match self.destination {
            IpAddr::V4(_) => self.encode_icmp_v4(payload),
            IpAddr::V6(_) => self.encode_icmp_v6(payload),
        }
    }

    /// Encodes as an ICMPv4 EchoRequest.
    fn encode_icmp_v4(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let req = PacketEchoRequest {
            ident: self.ident,
            seq_cnt: self.seq_cnt,
        };
        let mut buffer = vec![0; 8 + payload.len()];
        req.encode::<IcmpV4>(&mut buffer, payload)
    }

    /// Encodes as an ICMPv6 EchoRequest.
    fn encode_icmp_v6(&self, payload: &[u8]) -> Result<Vec<u8>> {
        let req = PacketEchoRequest {
            ident: self.ident,
            seq_cnt: self.seq_cnt,
        };
        let mut buffer = vec![0; 8 + payload.len()];
        req.encode::<IcmpV6>(&mut buffer, payload)
    }
}

/// `EchoReply` struct, which contains some packet information.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct EchoReply {
    /// IP Time To Live for outgoing packets. Present for ICMPv4 replies,
    /// absent for ICMPv6 replies.
    pub ttl: Option<u8>,
    /// Source address of ICMP packet.
    pub source: IpAddr,
    /// Sequence of ICMP packet.
    pub sequence: u16,
    /// Identifier of ICMP packet.
    pub identifier: u16,
    /// Size of ICMP echo payload.
    pub payload_len: usize,
    /// ICMP echo payload.
    pub payload: Vec<u8>,
    /// Deprecated alias for `payload_len`.
    #[deprecated(since = "0.6.0", note = "use payload_len instead")]
    pub size: usize,
}

impl EchoReply {
    /// Unpack IP packets received from socket as `EchoReply` struct.
    pub fn decode(addr: IpAddr, buf: &[u8]) -> Result<EchoReply> {
        Self::decode_raw(addr, buf)
    }

    pub(crate) fn decode_raw(addr: IpAddr, buf: &[u8]) -> Result<EchoReply> {
        match addr {
            IpAddr::V4(_) => decode_icmpv4(addr, buf),
            IpAddr::V6(_) => decode_icmpv6(addr, buf),
        }
    }

    pub(crate) fn decode_dgram(source: IpAddr, buf: &[u8]) -> Result<EchoReply> {
        match source {
            IpAddr::V4(_) => decode_icmpv4_dgram(source, buf),
            IpAddr::V6(_) => decode_icmpv6(source, buf),
        }
    }
}

/// Decodes an ICMPv4 packet received from an IPv4 raw socket
fn decode_icmpv4(_addr: IpAddr, buf: &[u8]) -> Result<EchoReply> {
    let ipv4_decoded = IpV4Packet::decode(buf)?;
    let source = ipv4_decoded.source;
    let ttl = Some(ipv4_decoded.ttl);
    let icmp_decoded = PacketEchoReply::decode::<IcmpV4>(ipv4_decoded.data)?;
    Ok(reply_from_packet(source, ttl, icmp_decoded))
}

/// Decodes an ICMPv4 packet received from an IPv4 datagram socket.
fn decode_icmpv4_dgram(source: IpAddr, buf: &[u8]) -> Result<EchoReply> {
    let icmp_decoded = PacketEchoReply::decode::<IcmpV4>(buf)?;
    Ok(reply_from_packet(source, None, icmp_decoded))
}

/// Decodes an ICMPv6 packet received from an IPv6 raw socket
fn decode_icmpv6(source: IpAddr, buf: &[u8]) -> Result<EchoReply> {
    let icmp_decoded = PacketEchoReply::decode::<IcmpV6>(buf)?;
    Ok(reply_from_packet(source, None, icmp_decoded))
}

#[allow(deprecated)]
fn reply_from_packet(source: IpAddr, ttl: Option<u8>, packet: PacketEchoReply<'_>) -> EchoReply {
    EchoReply {
        ttl,
        source,
        sequence: packet.seq_cnt,
        identifier: packet.ident,
        payload_len: packet.payload.len(),
        payload: packet.payload.to_vec(),
        size: packet.payload.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn decodes_raw_ipv4_reply_with_header_metadata() {
        let packet = [
            0x45, 0, 0, 30, 0, 0, 0, 0, 42, 1, 0, 0, 203, 0, 113, 9, 8, 8, 8, 8, 0, 0, 0, 0, 0x12,
            0x34, 0, 7, b'o', b'k',
        ];

        let reply = EchoReply::decode_raw(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), &packet).unwrap();

        assert_eq!(reply.source, IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9)));
        assert_eq!(reply.ttl, Some(42));
        assert_eq!(reply.identifier, 0x1234);
        assert_eq!(reply.sequence, 7);
        assert_eq!(reply.payload, b"ok");
        assert_eq!(reply.payload_len, 2);
    }

    #[test]
    fn decodes_dgram_ipv4_reply_without_ip_header() {
        let packet = [0, 0, 0, 0, 0x12, 0x34, 0, 7, b'o', b'k'];
        let source = IpAddr::V4(Ipv4Addr::new(203, 0, 113, 9));

        let reply = EchoReply::decode_dgram(source, &packet).unwrap();

        assert_eq!(reply.source, source);
        assert_eq!(reply.ttl, None);
        assert_eq!(reply.identifier, 0x1234);
        assert_eq!(reply.sequence, 7);
        assert_eq!(reply.payload, b"ok");
    }

    #[test]
    fn decodes_ipv6_reply_without_ip_header() {
        let source = "2001:db8::1".parse().unwrap();
        let packet = [129, 0, 0, 0, 0x12, 0x34, 0, 7, b'o', b'k'];

        let reply = EchoReply::decode_raw(source, &packet).unwrap();

        assert_eq!(reply.source, source);
        assert_eq!(reply.ttl, None);
        assert_eq!(reply.identifier, 0x1234);
        assert_eq!(reply.sequence, 7);
        assert_eq!(reply.payload, b"ok");
    }
}
