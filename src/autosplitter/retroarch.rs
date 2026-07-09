//! Client for RetroArch's Network Command Interface (`network_cmd_enable`
//! in `retroarch.cfg`, UDP port `55355` by default) — specifically the
//! `READ_CORE_MEMORY` command, which is all this needs.

use std::io;
use std::net::UdpSocket;
use std::time::Duration;

pub struct RetroArchClient {
    socket: UdpSocket,
}

impl RetroArchClient {
    pub fn connect(host: &str, port: u16) -> io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.connect((host, port))?;
        socket.set_read_timeout(Some(Duration::from_millis(500)))?;
        Ok(Self { socket })
    }

    pub fn read_memory(&self, address: u64, size: usize) -> io::Result<Vec<u8>> {
        let request = format!("READ_CORE_MEMORY {address:x} {size}\n");
        self.socket.send(request.as_bytes())?;

        let mut buf = [0u8; 4096];
        let n = self.socket.recv(&mut buf)?;
        let response = String::from_utf8_lossy(&buf[..n]).into_owned();

        parse_read_memory_response(&response).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unexpected RetroArch response: {response:?}"),
            )
        })
    }
}

/// Parses a `READ_CORE_MEMORY` response line: `READ_CORE_MEMORY <addr_hex>
/// <byte_hex> <byte_hex> ...`, or a trailing `-1` in place of the byte list
/// if the read failed (bad address, no core loaded, etc). Split out from
/// `RetroArchClient::read_memory` so it's testable without a real socket.
pub fn parse_read_memory_response(line: &str) -> Option<Vec<u8>> {
    let mut parts = line.split_whitespace();
    if parts.next()? != "READ_CORE_MEMORY" {
        return None;
    }
    let _address = parts.next()?; // echoed back; caller already knows it

    let rest: Vec<&str> = parts.collect();
    if rest.is_empty() || rest == ["-1"] {
        return None;
    }

    rest.iter()
        .map(|byte| u8::from_str_radix(byte, 16).ok())
        .collect()
}
