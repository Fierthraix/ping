# rust tiny-ping

[![Crates.io](https://img.shields.io/crates/v/tiny-ping.svg)](https://crates.io/crates/tiny-ping)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Docs](https://docs.rs/tiny-ping/badge.svg)](https://docs.rs/tiny-ping/)

Ping function implemented in rust, made for small compile times.

Small async ICMP library. No proc macros.

## Usage

```rust
use std::{net::IpAddr, time::Duration};

use tiny_ping::Pinger;

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let mut pinger = Pinger::new("1.1.1.1".parse::<IpAddr>()?)?;
pinger.timeout(Duration::from_secs(1));

let result = pinger.ping(1).await?;
println!("reply from {} in {:?}", result.reply.source, result.rtt);
# Ok(())
# }
```

For multicast or other cases where more than one host may reply to a single
request, use `ping_replies`:

```rust
use std::{net::{Ipv6Addr, SocketAddr, SocketAddrV6}, time::Duration};

use tiny_ping::{Pinger, SocketType};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let target = SocketAddr::V6(SocketAddrV6::new(
    "ff02::1".parse::<Ipv6Addr>()?,
    0,
    0,
    2,
));
let mut pinger = Pinger::with_socket_addr(target, SocketType::Raw)?;
pinger.timeout(Duration::from_secs(1));

for result in pinger.ping_replies(1).await? {
    println!("reply from {} in {:?}", result.reply.source, result.rtt);
}
# Ok(())
# }
```

Raw sockets usually need root or capabilities.

DGRAM sockets can work without that on some systems:

```rust
use std::net::IpAddr;

use tiny_ping::{Pinger, SocketType};

# async fn example() -> Result<(), Box<dyn std::error::Error>> {
let pinger = Pinger::with_socket_type(
    "1.1.1.1".parse::<IpAddr>()?,
    SocketType::Dgram,
)?;

let result = pinger.ping(1).await?;
println!("reply from {} in {:?}", result.reply.source, result.rtt);
# Ok(())
# }
```

## Tests

```sh
cargo test
```

Real ping tests are opt-in with `TINY_PING_RUN_NET_TESTS=1` or
`TINY_PING_RUN_RAW_TESTS=1`.

## License

This library contains codes from https://github.com/knsd/tokio-ping, which is licensed under either of

- Apache License, Version 2.0 (LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)

And other codes is licensed under

- MIT license (LICENSE-MIT or http://opensource.org/licenses/MIT)
