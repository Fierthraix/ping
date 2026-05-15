use std::{net::IpAddr, time::Duration};

use tiny_ping::{Pinger, SocketType};

#[test]
fn raw_loopback_ping_when_enabled() {
    if std::env::var_os("TINY_PING_RUN_RAW_TESTS").is_none() {
        return;
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut pinger = Pinger::new("127.0.0.1".parse::<IpAddr>().unwrap()).unwrap();
        pinger.timeout(Duration::from_secs(1));
        let result = pinger.ping(1).await.unwrap();

        assert_eq!(result.socket_type, SocketType::Raw);
        assert_eq!(result.reply.sequence, 1);
    });
}

#[test]
fn dgram_loopback_ping_when_enabled_and_supported() {
    if std::env::var_os("TINY_PING_RUN_NET_TESTS").is_none() {
        return;
    }

    let Ok(mut pinger) =
        Pinger::with_socket_type("127.0.0.1".parse::<IpAddr>().unwrap(), SocketType::Dgram)
    else {
        return;
    };

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        pinger.timeout(Duration::from_secs(1));
        let result = pinger.ping(1).await.unwrap();

        assert_eq!(result.socket_type, SocketType::Dgram);
        assert_eq!(result.reply.sequence, 1);
    });
}
