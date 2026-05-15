pub type Result<T> = std::result::Result<T, Error>;

/// An error resulting from a ping option-setting or send/receive operation.
#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    IncorrectBufferSize,
    NotIpv4Packet,
    NotIcmpPacket,
    NotIcmpv6Packet,
    PayloadTooShort { got: usize, want: usize },
    IOError(String),
    NotEchoReply,
    NotV6EchoReply,
    Timeout,
    OtherICMP,
    InvalidSize,
    InvalidPacket,
    TooSmallHeader,
    InvalidHeaderSize,
    InvalidVersion,
    UnknownProtocol,
    Unimplemented,
    UnsupportedSocketType,
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io(error) => write!(f, "io error: {error}"),
            Error::IncorrectBufferSize => write!(f, "incorrect buffer size"),
            Error::NotIpv4Packet => write!(f, "not an IPv4 packet"),
            Error::NotIcmpPacket => write!(f, "not an ICMP packet"),
            Error::NotIcmpv6Packet => write!(f, "not an ICMPv6 packet"),
            Error::PayloadTooShort { got, want } => {
                write!(f, "payload too short: got {got} bytes, want {want}")
            }
            Error::IOError(error) => write!(f, "io error: {error}"),
            Error::NotEchoReply => write!(f, "not an ICMP echo reply"),
            Error::NotV6EchoReply => write!(f, "not an ICMPv6 echo reply"),
            Error::Timeout => write!(f, "ping timed out"),
            Error::OtherICMP => write!(f, "received a non-echo ICMP packet"),
            Error::InvalidSize => write!(f, "invalid packet size"),
            Error::InvalidPacket => write!(f, "invalid packet"),
            Error::TooSmallHeader => write!(f, "packet header is too small"),
            Error::InvalidHeaderSize => write!(f, "invalid IP header size"),
            Error::InvalidVersion => write!(f, "invalid IP version"),
            Error::UnknownProtocol => write!(f, "unknown IP protocol"),
            Error::Unimplemented => write!(f, "operation is not implemented"),
            Error::UnsupportedSocketType => write!(f, "unsupported socket type"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Io(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_not_empty() {
        let errors = [
            Error::Timeout,
            Error::InvalidPacket,
            Error::InvalidHeaderSize,
            Error::UnsupportedSocketType,
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
