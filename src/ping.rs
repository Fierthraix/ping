use std::{
    mem::MaybeUninit,
    net::{IpAddr, SocketAddr},
    sync::atomic::{AtomicU16, Ordering},
    time::{Duration, Instant},
};

use tokio::time::timeout;

use crate::error::{Error, Result};
use crate::icmp::{EchoReply, EchoRequest};
use crate::socket::AsyncSocket;

pub use crate::socket::SocketType;

const DEFAULT_PAYLOAD_SIZE: usize = 56;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(2);
const TOKEN_SIZE: usize = 8;

static NEXT_IDENT: AtomicU16 = AtomicU16::new(1);

#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct PingResult {
    pub reply: EchoReply,
    pub rtt: Duration,
    pub socket_type: SocketType,
}

/// A Ping struct represents the state of one particular ping instance.
#[derive(Debug, Clone)]
pub struct Pinger {
    target: SocketAddr,
    ident: u16,
    size: usize,
    timeout: Duration,
    ttl: Option<u32>,
    socket: AsyncSocket,
}

impl Pinger {
    /// Creates a new raw-socket ping instance from `IpAddr`.
    pub fn new(host: IpAddr) -> Result<Pinger> {
        Self::with_socket_type(host, SocketType::Raw)
    }

    /// Creates a new ping instance using a specific socket type.
    pub fn with_socket_type(host: IpAddr, socket_type: SocketType) -> Result<Pinger> {
        Self::with_socket_addr(SocketAddr::new(host, 0), socket_type)
    }

    /// Creates a new ping instance using a specific socket address and socket type.
    ///
    /// The port is ignored. For IPv6, callers can use this to provide a
    /// `SocketAddrV6` scope ID, for example when targeting link-local multicast.
    pub fn with_socket_addr(target: SocketAddr, socket_type: SocketType) -> Result<Pinger> {
        Ok(Pinger {
            target,
            ident: default_ident(),
            size: DEFAULT_PAYLOAD_SIZE,
            timeout: DEFAULT_TIMEOUT,
            ttl: None,
            socket: AsyncSocket::new(target.ip(), socket_type)?,
        })
    }

    /// Changes the socket type and recreates the underlying socket.
    pub fn socket_type(&mut self, socket_type: SocketType) -> Result<&mut Pinger> {
        let socket = AsyncSocket::new(self.target.ip(), socket_type)?;
        if let Some(ttl) = self.ttl {
            socket.set_ttl(self.target.ip(), ttl)?;
        }
        self.socket = socket;
        Ok(self)
    }

    /// Returns the active socket type.
    pub fn active_socket_type(&self) -> SocketType {
        self.socket.socket_type()
    }

    /// Sets the value for the `SO_BINDTODEVICE` option on this socket.
    ///
    /// If a socket is bound to an interface, only packets received from that
    /// particular interface are processed by the socket. Note that this only
    /// works for some socket types, particularly `AF_INET` sockets.
    ///
    /// If `interface` is `None` or an empty string it removes the binding.
    ///
    /// This function is only available on Fuchsia and Linux.
    #[cfg(any(target_os = "android", target_os = "fuchsia", target_os = "linux"))]
    pub fn bind_device(&mut self, interface: Option<&[u8]>) -> Result<&mut Pinger> {
        self.socket.bind_device(interface)?;
        Ok(self)
    }

    /// Set the identification of ICMP.
    pub fn ident(&mut self, val: u16) -> &mut Pinger {
        self.ident = val;
        self
    }

    /// Set the packet payload size in bytes. (default: 56)
    pub fn size(&mut self, size: usize) -> &mut Pinger {
        self.size = size;
        self
    }

    /// Set the timeout of each ping. (default: 2s)
    pub fn timeout(&mut self, timeout: Duration) -> &mut Pinger {
        self.timeout = timeout;
        self
    }

    /// Set the outgoing IPv4 TTL or IPv6 unicast hop limit.
    pub fn ttl(&mut self, ttl: u32) -> Result<&mut Pinger> {
        self.socket.set_ttl(self.target.ip(), ttl)?;
        self.ttl = Some(ttl);
        Ok(self)
    }

    async fn recv_reply(&self, seq_cnt: u16, payload: &[u8]) -> Result<EchoReply> {
        let mut buffer = [MaybeUninit::new(0); 2048];
        loop {
            let (size, source) = self.socket.recv_from(&mut buffer).await?;
            let buf = unsafe { assume_init(&buffer[..size]) };
            let source = source.map(|addr| addr.ip()).unwrap_or(self.target.ip());
            let decoded = match self.socket.socket_type() {
                SocketType::Raw if self.target.ip().is_ipv6() => EchoReply::decode_raw(source, buf),
                SocketType::Raw => EchoReply::decode_raw(self.target.ip(), buf),
                SocketType::Dgram => EchoReply::decode_dgram(source, buf),
            };

            match decoded {
                Ok(reply) if self.reply_matches(&reply, seq_cnt, payload) => return Ok(reply),
                Ok(_) => continue,
                Err(Error::InvalidPacket)
                | Err(Error::NotEchoReply)
                | Err(Error::NotV6EchoReply)
                | Err(Error::OtherICMP)
                | Err(Error::UnknownProtocol) => continue,
                Err(e) => return Err(e),
            }
        }
    }

    fn reply_matches(&self, reply: &EchoReply, seq_cnt: u16, payload: &[u8]) -> bool {
        if reply.sequence != seq_cnt {
            return false;
        }

        if self.socket.socket_type() == SocketType::Raw && reply.identifier != self.ident {
            return false;
        }

        payload.is_empty() || reply.payload == payload
    }

    async fn send_request(&self, seq_cnt: u16, payload: &[u8]) -> Result<Instant> {
        let packet =
            EchoRequest::new(self.target.ip(), self.ident, seq_cnt).encode_with_payload(payload)?;

        let sent = Instant::now();
        let size = self.socket.send_to(&packet, &self.target.into()).await?;
        if size != packet.len() {
            return Err(Error::InvalidSize);
        }

        Ok(sent)
    }

    /// Send a ping request with sequence number.
    pub async fn ping(&self, seq_cnt: u16) -> Result<PingResult> {
        let payload = request_payload(self.ident, seq_cnt, self.size);
        let sent = self.send_request(seq_cnt, &payload).await?;

        let reply = timeout(self.timeout, self.recv_reply(seq_cnt, &payload))
            .await
            .map_err(|_| Error::Timeout)??;

        Ok(PingResult {
            reply,
            rtt: sent.elapsed(),
            socket_type: self.socket.socket_type(),
        })
    }

    /// Send one ping request and collect all matching replies until timeout.
    ///
    /// This is useful for multicast targets where more than one host can reply
    /// to the same echo request. Unlike [`Pinger::ping`], a timeout after the
    /// request is sent is not an error; it ends collection and returns the
    /// replies seen so far.
    pub async fn ping_replies(&self, seq_cnt: u16) -> Result<Vec<PingResult>> {
        let payload = request_payload(self.ident, seq_cnt, self.size);
        let sent = self.send_request(seq_cnt, &payload).await?;
        let deadline = sent + self.timeout;
        let mut replies = Vec::new();

        while let Some(remaining) = deadline.checked_duration_since(Instant::now()) {
            let reply = match timeout(remaining, self.recv_reply(seq_cnt, &payload)).await {
                Ok(reply) => reply?,
                Err(_) => break,
            };

            replies.push(PingResult {
                reply,
                rtt: sent.elapsed(),
                socket_type: self.socket.socket_type(),
            });
        }

        Ok(replies)
    }
}

fn default_ident() -> u16 {
    let pid = std::process::id() as u16;
    let next = NEXT_IDENT.fetch_add(1, Ordering::Relaxed);
    pid.wrapping_add(next)
}

fn request_payload(ident: u16, seq_cnt: u16, size: usize) -> Vec<u8> {
    let mut payload = vec![0; size];
    let token = [
        b't',
        b'p',
        (ident >> 8) as u8,
        ident as u8,
        (seq_cnt >> 8) as u8,
        seq_cnt as u8,
        (size >> 8) as u8,
        size as u8,
    ];
    let len = payload.len().min(TOKEN_SIZE);
    payload[..len].copy_from_slice(&token[..len]);
    payload
}

/// Assume the `buf`fer to be initialised.
///
/// # Safety
///
/// `socket2` initialises exactly the number of bytes returned by `recv_from`.
unsafe fn assume_init(buf: &[MaybeUninit<u8>]) -> &[u8] {
    unsafe { &*(buf as *const [MaybeUninit<u8>] as *const [u8]) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_payload_respects_size() {
        assert_eq!(request_payload(1, 2, 0), Vec::<u8>::new());
        assert_eq!(request_payload(1, 2, 4), vec![b't', b'p', 0, 1]);
        assert_eq!(request_payload(1, 2, 8), vec![b't', b'p', 0, 1, 0, 2, 0, 8]);
    }
}
