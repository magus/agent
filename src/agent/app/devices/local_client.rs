use super::device_service::{DeviceService, RemoteRequest, response};
use super::transport;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io;
use std::net::SocketAddr;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub(crate) fn run(path: &str) -> io::Result<()> {
    let request: LocalRequest = serde_json::from_slice(&fs::read(path)?)?;
    let runtime_dir = crate::app::core::runtime_dir()?;
    let mut stream = UnixStream::connect(runtime_dir.join("devices.sock"))?;
    stream.set_read_timeout(Some(Duration::from_secs(40)))?;
    transport::send(&mut stream, &request)?;
    let response: Value = transport::receive(&mut stream)?;
    println!("{}", serde_json::to_string_pretty(&response)?);
    if response["ok"] != true {
        return Err(io::Error::other("Device command failed"));
    }
    Ok(())
}

pub(super) fn start(service: Arc<DeviceService>, runtime_dir: &Path) -> io::Result<()> {
    let path = runtime_dir.join("devices.sock");
    if let Ok(metadata) = fs::symlink_metadata(&path) {
        if UnixStream::connect(&path).is_ok() {
            return Err(io::Error::other("Agent device service is already running"));
        }
        if !metadata.file_type().is_socket() || metadata.uid() != unsafe { libc::getuid() } {
            return Err(io::Error::other("Unexpected device socket path"));
        }
        fs::remove_file(&path)?;
    }
    let listener = UnixListener::bind(&path)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    std::thread::Builder::new()
        .name("device-admin".into())
        .spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let service = service.clone();
                std::thread::spawn(move || {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(40)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(40)));
                    let result = transport::receive::<LocalRequest>(&mut stream)
                        .and_then(|request| dispatch(&service, request));
                    let _ = transport::send(&mut stream, &response(result));
                });
            }
        })?;
    Ok(())
}

fn dispatch(service: &Arc<DeviceService>, request: LocalRequest) -> io::Result<Value> {
    match request {
        LocalRequest::List => Ok(service.snapshot()),
        LocalRequest::Configure { address, name } => service.configure(address, name),
        LocalRequest::Invite => service.invite(),
        LocalRequest::Pair { invitation } => service.pair(&invitation),
        LocalRequest::Approve { id } => service.approve(&id),
        LocalRequest::RevokeAccess { id } => service.revoke_access(&id),
        LocalRequest::RemoveTarget { id } => service.remove_target(&id),
        LocalRequest::Decline { id } => service.decline(&id),
        LocalRequest::AllowRequests { id } => service.allow_requests(&id),
        LocalRequest::Rename { name } => service.rename(name),
        LocalRequest::Discover => service.discover(),
        LocalRequest::RequestAccess { id } => service.request_access(&id),
        LocalRequest::Call { host, request } => service.call(&host, request),
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum LocalRequest {
    List,
    Configure {
        address: SocketAddr,
        name: String,
    },
    Invite,
    Pair {
        invitation: String,
    },
    Approve {
        id: String,
    },
    RevokeAccess {
        id: String,
    },
    RemoveTarget {
        id: String,
    },
    Decline {
        id: String,
    },
    AllowRequests {
        id: String,
    },
    Rename {
        name: String,
    },
    Discover,
    RequestAccess {
        id: String,
    },
    Call {
        host: String,
        request: RemoteRequest,
    },
}
