use super::device_store::Device;
use super::{tailscale, transport};
use serde::{Deserialize, Serialize};
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

pub(super) fn query(address: SocketAddr, node_id: &str) -> io::Result<Description> {
    if tailscale::identify(address)? != node_id {
        return Err(io::Error::other("Tailscale device identity changed"));
    }
    let mut socket = TcpStream::connect_timeout(&address, Duration::from_secs(2))?;
    socket.set_read_timeout(Some(Duration::from_secs(3)))?;
    socket.set_write_timeout(Some(Duration::from_secs(3)))?;
    // Only public identity metadata uses this route, inside the authenticated Tailscale tunnel.
    socket.write_all(b"AGENT-ID\n")?;
    let description: Description = transport::receive_limited(&mut socket, 32 * 1024)?;
    if description.device.address != address || tailscale::identify(address)? != node_id {
        return Err(io::Error::other(
            "Device address or Tailscale identity changed",
        ));
    }
    Ok(description)
}

pub(super) fn answer(socket: &mut TcpStream, description: &Description) -> io::Result<()> {
    tailscale::identify(socket.peer_addr()?)?;
    let mut preface = [0; 9];
    socket.read_exact(&mut preface)?;
    if &preface != b"AGENT-ID\n" {
        return Err(io::Error::other("Unknown discovery request"));
    }
    transport::send(socket, description)
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Description {
    pub device: Device,
    pub tool_list: Vec<String>,
}

#[derive(Clone, Serialize)]
pub(super) struct Candidate {
    pub description: Description,
    pub node_id: String,
}
