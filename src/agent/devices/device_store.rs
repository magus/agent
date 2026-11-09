use super::identity::device_id;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::net::SocketAddr;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Device {
    pub name: String,
    pub address: SocketAddr,
    pub certificate: Vec<u8>,
}

impl Device {
    pub fn id(&self) -> String {
        device_id(&self.certificate)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Peer {
    pub device: Device,
    pub may_control_us: bool,
    pub target: bool,
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub(super) struct DeviceStore {
    pub address: Option<SocketAddr>,
    pub name: String,
    pub peer_list: Vec<Peer>,
    #[serde(default)]
    pub name_override: Option<String>,
    #[serde(default)]
    pub name_migrated: bool,
    #[serde(default)]
    pub declined_id_list: Vec<String>,
}

impl DeviceStore {
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }

    pub fn peer(&self, host: &str) -> io::Result<Peer> {
        let mut match_list = self
            .peer_list
            .iter()
            .filter(|peer| peer.target && (peer.device.id() == host || peer.device.name == host));
        let peer = match_list
            .next()
            .ok_or_else(|| io::Error::other("Unknown target host"))?;
        if match_list.next().is_some() {
            return Err(io::Error::other(
                "Host name is ambiguous; use its device ID",
            ));
        }
        Ok(peer.clone())
    }
}

pub(super) fn save_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec_pretty(value)?)?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    if let Some(parent) = path.parent() {
        fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}
