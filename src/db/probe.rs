//! Local server availability helpers.

use super::provision::LocalStatus;
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

pub fn tcp_localhost(port: u16) -> LocalStatus {
    let addr = format!("127.0.0.1:{port}");
    match try_connect(&addr) {
        true => LocalStatus::Ready {
            host: "127.0.0.1".into(),
            port,
        },
        false => LocalStatus::Missing,
    }
}

fn try_connect(addr: &str) -> bool {
    let Ok(mut addrs) = addr.to_socket_addrs() else {
        return false;
    };
    let Some(sock): Option<SocketAddr> = addrs.next() else {
        return false;
    };
    TcpStream::connect_timeout(&sock, Duration::from_millis(400)).is_ok()
}
