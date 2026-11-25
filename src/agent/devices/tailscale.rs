use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::net::{IpAddr, SocketAddr};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub(super) fn status() -> io::Result<Status> {
    let status: Status = serde_json::from_value(command(&["status", "--json"])?)?;
    if status.backend_state != "Running" {
        return Err(io::Error::other("Connect Tailscale to find devices"));
    }
    Ok(status)
}

pub(super) fn identify(address: SocketAddr) -> io::Result<String> {
    // whois also exits nonzero for an address that is not on the tailnet, so only blame Tailscale
    // when its own status says it is not running.
    let result = command(&["whois", "--json", &address.ip().to_string()]).map_err(|error| {
        if error.to_string() != "Tailscale is unavailable" {
            return error;
        }
        match status() {
            Ok(_) => io::Error::other("No device on your tailnet has this address"),
            Err(status_error) => status_error,
        }
    })?;
    let id = result["Node"]["StableID"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| io::Error::other("Tailscale could not identify this device"))?;
    let expected_address = format!("{}/32", address.ip());
    let direct_address = result["Node"]["Addresses"].as_array().is_some_and(|list| {
        list.iter()
            .any(|value| value.as_str() == Some(&expected_address))
    });
    if !direct_address {
        return Err(io::Error::other("Use a direct Tailscale device address"));
    }
    Ok(id.to_string())
}

pub(super) fn resolve_name(host: &str) -> io::Result<SocketAddr> {
    let status = status()?;
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    let mut address_list = Vec::new();
    for node in status.peer_by_key.values() {
        let dns_name = node.dns_name.trim_end_matches('.').to_ascii_lowercase();
        let short_name = dns_name.split('.').next().unwrap_or("");
        if host == dns_name || host == short_name || host == node.name.to_ascii_lowercase() {
            address_list.push(node.address()?);
        }
    }
    address_list.sort();
    address_list.dedup();
    match address_list.as_slice() {
        [address] => Ok(*address),
        [] => Err(io::Error::other(
            "Hostname not found. Enter its IP address or use Pair manually.",
        )),
        _ => Err(io::Error::other(
            "Hostname matches multiple devices. Enter its full hostname or IP address.",
        )),
    }
}

#[derive(Deserialize)]
pub(super) struct Status {
    #[serde(rename = "BackendState")]
    backend_state: String,
    #[cfg(not(target_os = "ios"))]
    #[serde(rename = "Self")]
    pub this_device: Node,
    #[serde(rename = "Peer", default)]
    pub peer_by_key: HashMap<String, Node>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Node {
    #[cfg(not(target_os = "ios"))]
    #[serde(rename = "ID")]
    pub id: String,
    #[serde(rename = "HostName")]
    pub name: String,
    #[serde(rename = "DNSName", default)]
    dns_name: String,
    #[serde(rename = "TailscaleIPs", default)]
    address_list: Vec<IpAddr>,
    #[cfg(not(target_os = "ios"))]
    #[serde(rename = "Online", default)]
    pub online: bool,
}

impl Node {
    pub fn address(&self) -> io::Result<SocketAddr> {
        self.address_list
            .iter()
            .find_map(|ip| match ip {
                IpAddr::V4(ip) if ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]) => {
                    Some(SocketAddr::new(IpAddr::V4(*ip), 47321))
                }
                _ => None,
            })
            .ok_or_else(|| io::Error::other("Tailscale has no device address"))
    }
}

fn command(argument_list: &[&str]) -> io::Result<Value> {
    let executable = [
        "/opt/homebrew/bin/tailscale",
        "/usr/local/bin/tailscale",
        "/Applications/Tailscale.app/Contents/MacOS/Tailscale",
    ]
    .into_iter()
    .find(|path| std::path::Path::new(path).is_file())
    .ok_or_else(|| io::Error::other("Install Tailscale to connect devices"))?;
    let mut child = Command::new(executable)
        .args(argument_list)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let mut stdout = child.stdout.take().unwrap();
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        stdout
            .by_ref()
            .take(4 * 1024 * 1024)
            .read_to_end(&mut bytes)?;
        io::Result::Ok(bytes)
    });
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait()? {
            let bytes = reader
                .join()
                .map_err(|_| io::Error::other("Tailscale read failed"))??;
            if !status.success() {
                return Err(io::Error::other("Tailscale is unavailable"));
            }
            return serde_json::from_slice(&bytes).map_err(io::Error::other);
        }
        if started.elapsed() > Duration::from_secs(5) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("Tailscale did not respond"));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
