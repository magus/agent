use super::device_store::{Device, DeviceStore, Peer, save_json};
use super::identity::{Identity, device_id};
use super::tool_store::{Tool, ToolStore};
use super::transport;
use super::{discovery, tailscale};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, TcpListener};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) struct DeviceService {
    identity: Identity,
    store: Mutex<DeviceStore>,
    store_path: PathBuf,
    invitation_by_secret: Mutex<HashMap<String, InvitationState>>,
    pending_by_id: Mutex<HashMap<String, Device>>,
    listening: Mutex<Option<SocketAddr>>,
    candidate_by_id: Mutex<HashMap<String, discovery::Candidate>>,
    status_by_id: Mutex<HashMap<String, String>>,
    connection_status: Mutex<String>,
    pub tools: Arc<ToolStore>,
}

impl DeviceService {
    pub fn load(runtime_dir: PathBuf) -> io::Result<Arc<Self>> {
        let store_path = runtime_dir.join("devices.json");
        let service = Arc::new(Self {
            identity: Identity::load()?,
            store: Mutex::new(DeviceStore::load(&store_path)?),
            store_path,
            invitation_by_secret: Mutex::new(HashMap::new()),
            pending_by_id: Mutex::new(HashMap::new()),
            listening: Mutex::new(None),
            candidate_by_id: Mutex::new(HashMap::new()),
            status_by_id: Mutex::new(HashMap::new()),
            connection_status: Mutex::new("Finding Tailscale".into()),
            tools: ToolStore::load(runtime_dir)?,
        });
        let background = service.clone();
        std::thread::spawn(move || {
            loop {
                if let Err(error) = background.discover() {
                    *background.connection_status.lock().unwrap() = error.to_string();
                }
                std::thread::sleep(Duration::from_secs(15));
            }
        });
        let background = service.clone();
        std::thread::spawn(move || {
            loop {
                background.refresh_target_status();
                std::thread::sleep(Duration::from_secs(2));
            }
        });
        Ok(service)
    }

    pub fn snapshot(&self) -> Value {
        let mut candidate_list = self
            .candidate_by_id
            .lock()
            .unwrap()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        candidate_list.sort_by(|first, second| {
            first
                .description
                .device
                .name
                .cmp(&second.description.device.name)
        });
        let mut pending_list = self
            .pending_by_id
            .lock()
            .unwrap()
            .iter()
            .map(|(id, device)| json!({"id": id, "device": device}))
            .collect::<Vec<_>>();
        pending_list
            .sort_by_key(|pending| pending["device"]["name"].as_str().unwrap_or("").to_string());
        let store = self.store.lock().unwrap();
        json!({"id": device_id(&self.identity.certificate), "name": store.name,
            "address": *self.listening.lock().unwrap(), "configured_address": store.address,
            "peer_list": store.peer_list,
            "declined_id_list": store.declined_id_list,
            "candidate_list": candidate_list,
            "status_by_id": *self.status_by_id.lock().unwrap(),
            "connection_status": *self.connection_status.lock().unwrap(),
            "pending_list": pending_list,
            "tool_list": ["shell", "computer"]})
    }

    pub fn rename(&self, name: String) -> io::Result<Value> {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(io::Error::other("Use a device name with 1–100 characters"));
        }
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        updated.name = name.into();
        save_json(&self.store_path, &updated)?;
        *store = updated;
        Ok(json!({"renamed": name}))
    }

    pub fn discover(self: &Arc<Self>) -> io::Result<Value> {
        let status = tailscale::status()?;
        let address = status.this_device.address()?;
        self.listen(address)?;
        {
            let mut store = self.store.lock().unwrap();
            if store.address != Some(address) || store.name.is_empty() {
                store.address = Some(address);
                if store.name.is_empty() {
                    store.name = status.this_device.name;
                }
                save_json(&self.store_path, &*store)?;
            }
        }
        *self.connection_status.lock().unwrap() = "Connected through Tailscale".into();
        let mut worker_list = Vec::new();
        for node in status
            .peer_by_key
            .into_values()
            .filter(|node| node.online)
            .take(128)
        {
            worker_list.push(std::thread::spawn(move || {
                let address = node.address()?;
                let description = discovery::query(address, &node.id)?;
                // A pinned TLS handshake proves that Agent holds the certificate's private key.
                let mut connection =
                    transport::connect(address, description.device.certificate.clone(), None)?;
                transport::send(&mut connection, &RemoteRequest::Describe)?;
                let verified: discovery::Description =
                    serde_json::from_value(read_result(&mut connection)?)?;
                if verified.device.id() != description.device.id()
                    || verified.device.address != address
                {
                    return Err(io::Error::other("Agent identity changed during discovery"));
                }
                Ok(discovery::Candidate {
                    description: verified,
                    node_id: node.id,
                })
            }));
        }
        let mut candidate_by_id = HashMap::new();
        for worker in worker_list {
            if let Ok(Ok(candidate)) = worker.join() {
                candidate_by_id.insert(candidate.description.device.id(), candidate);
            }
        }
        {
            let mut store = self.store.lock().unwrap();
            let mut updated = store.clone();
            let mut changed = false;
            for peer in &mut updated.peer_list {
                if let Some(candidate) = candidate_by_id.get(&peer.device.id()) {
                    let device = &candidate.description.device;
                    if device.name != peer.device.name || device.address != peer.device.address {
                        peer.device = device.clone();
                        changed = true;
                    }
                }
            }
            if changed {
                save_json(&self.store_path, &updated)?;
                *store = updated;
            }
        }
        *self.candidate_by_id.lock().unwrap() = candidate_by_id;
        self.refresh_target_status();
        Ok(self.snapshot())
    }

    fn refresh_target_status(&self) {
        let peer_list = self.store.lock().unwrap().peer_list.clone();
        for peer in peer_list.into_iter().filter(|peer| peer.target) {
            let result = (|| {
                let mut connection =
                    transport::connect(peer.device.address, peer.device.certificate.clone(), None)?;
                transport::send(
                    &mut connection,
                    &RemoteRequest::PairingStatus {
                        id: device_id(&self.identity.certificate),
                    },
                )?;
                read_result(&mut connection)
            })();
            let status = match result {
                Ok(value) => value["status"]
                    .as_str()
                    .unwrap_or("Unavailable")
                    .to_string(),
                Err(_) => "Offline or unavailable".into(),
            };
            self.status_by_id
                .lock()
                .unwrap()
                .insert(peer.device.id(), status);
        }
    }

    pub fn request_access(&self, id: &str) -> io::Result<Value> {
        let candidate = self
            .candidate_by_id
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| {
                io::Error::other("Device is unavailable. Refresh devices and try again.")
            })?;
        let device = candidate.description.device;
        if tailscale::identify(device.address)? != candidate.node_id {
            return Err(io::Error::other("Tailscale device identity changed"));
        }
        let mut connection = transport::connect(device.address, device.certificate.clone(), None)?;
        transport::send(
            &mut connection,
            &RemoteRequest::RequestAccess {
                device: self.local_device()?,
            },
        )?;
        let result = read_result(&mut connection)?;
        self.save_target(device)?;
        self.refresh_target_status();
        Ok(result)
    }

    fn save_target(&self, device: Device) -> io::Result<()> {
        let mut store = self.store.lock().unwrap();
        if let Some(peer) = store
            .peer_list
            .iter_mut()
            .find(|peer| peer.device.id() == device.id())
        {
            peer.target = true;
            peer.device = device;
        } else {
            store.peer_list.push(Peer {
                device,
                may_control_us: false,
                target: true,
            });
        }
        save_json(&self.store_path, &*store)
    }

    pub fn configure(self: &Arc<Self>, address: SocketAddr, name: String) -> io::Result<Value> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(io::Error::other(
                "Device name must contain 1–100 characters",
            ));
        }
        self.listen(address)?;
        let mut store = self.store.lock().unwrap();
        store.name = name;
        store.address = Some(address);
        save_json(&self.store_path, &*store)?;
        drop(store);
        Ok(self.snapshot())
    }

    pub fn invite(&self) -> io::Result<Value> {
        let device = self.local_device()?;
        let secret = format!(
            "{}{}",
            uuid::Uuid::new_v4().simple(),
            uuid::Uuid::new_v4().simple()
        );
        let invitation = Invitation {
            device,
            secret: secret.clone(),
        };
        let mut invitation_by_secret = self.invitation_by_secret.lock().unwrap();
        invitation_by_secret
            .retain(|_, invitation| invitation.created.elapsed() < Duration::from_secs(600));
        invitation_by_secret.insert(
            secret,
            InvitationState {
                created: Instant::now(),
                claimed_by: None,
            },
        );
        let bytes = serde_json::to_vec(&invitation)?;
        Ok(
            json!({"invitation": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes),
            "expires_in_seconds": 600}),
        )
    }

    pub fn pair(&self, invitation: &str) -> io::Result<Value> {
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(invitation)
            .map_err(|_| io::Error::other("Invalid invitation"))?;
        let invitation: Invitation = serde_json::from_slice(&bytes).map_err(|_| {
            io::Error::other("Invalid invitation. Copy a new invitation from the other device.")
        })?;
        validate_address(invitation.device.address)?;
        let device = self.local_device()?;
        if device.id() == invitation.device.id() {
            return Err(io::Error::other("Cannot pair a device with itself"));
        }
        let mut connection = transport::connect(
            invitation.device.address,
            invitation.device.certificate.clone(),
            None,
        )?;
        transport::send(
            &mut connection,
            &RemoteRequest::Pair {
                secret: invitation.secret,
                device,
            },
        )?;
        let result = read_result(&mut connection)?;
        self.save_target(invitation.device)?;
        self.refresh_target_status();
        Ok(result)
    }

    pub fn approve(&self, id: &str) -> io::Result<Value> {
        let pending_device = self.pending_by_id.lock().unwrap().get(id).cloned();
        let device = pending_device.ok_or_else(|| io::Error::other("Pairing request not found"))?;
        let mut store = self.store.lock().unwrap();
        if let Some(peer) = store
            .peer_list
            .iter_mut()
            .find(|peer| peer.device.id() == id)
        {
            peer.may_control_us = true;
            peer.device = device;
        } else {
            store.peer_list.push(Peer {
                device,
                may_control_us: true,
                target: false,
            });
        }
        store.declined_id_list.retain(|declined| declined != id);
        save_json(&self.store_path, &*store)?;
        self.pending_by_id.lock().unwrap().remove(id);
        Ok(json!({"approved": id}))
    }

    pub fn revoke_access(&self, id: &str) -> io::Result<Value> {
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        if let Some(peer) = updated
            .peer_list
            .iter_mut()
            .find(|peer| peer.device.id() == id)
        {
            peer.may_control_us = false;
        }
        if !updated
            .declined_id_list
            .iter()
            .any(|declined| declined == id)
        {
            updated.declined_id_list.push(id.into());
        }
        save_json(&self.store_path, &updated)?;
        *store = updated;
        self.pending_by_id.lock().unwrap().remove(id);
        Ok(json!({"revoked_access": id}))
    }

    pub fn remove_target(&self, id: &str) -> io::Result<Value> {
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        if let Some(peer) = updated
            .peer_list
            .iter_mut()
            .find(|peer| peer.device.id() == id)
        {
            peer.target = false;
        }
        save_json(&self.store_path, &updated)?;
        *store = updated;
        self.status_by_id.lock().unwrap().remove(id);
        Ok(json!({"removed_target": id}))
    }

    pub fn allow_requests(&self, id: &str) -> io::Result<Value> {
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        updated.declined_id_list.retain(|declined| declined != id);
        save_json(&self.store_path, &updated)?;
        *store = updated;
        Ok(json!({"status": "New requests allowed. Control still requires approval."}))
    }

    pub fn decline(&self, id: &str) -> io::Result<Value> {
        let device = self
            .pending_by_id
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| io::Error::other("Pairing request not found"))?;
        {
            let mut store = self.store.lock().unwrap();
            if !store.peer_list.iter().any(|peer| peer.device.id() == id) {
                store.peer_list.push(Peer {
                    device,
                    may_control_us: false,
                    target: false,
                });
            }
        }
        self.revoke_access(id)?;
        Ok(json!({"status": "Declined"}))
    }

    pub fn call(&self, host: &str, request: RemoteRequest) -> io::Result<Value> {
        if matches!(
            request,
            RemoteRequest::Pair { .. } | RemoteRequest::RequestAccess { .. }
        ) {
            return Err(io::Error::other("Use an invitation to pair"));
        }
        let peer = self.store.lock().unwrap().peer(host)?;
        let mut connection = transport::connect(
            peer.device.address,
            peer.device.certificate,
            Some(&self.identity),
        )?;
        transport::send(&mut connection, &request)?;
        read_result(&mut connection)
    }

    fn local_device(&self) -> io::Result<Device> {
        let store = self.store.lock().unwrap();
        let address = self
            .listening
            .lock()
            .unwrap()
            .ok_or_else(|| io::Error::other("Connect Tailscale before pairing"))?;
        Ok(Device {
            name: store.name.clone(),
            address,
            certificate: self.identity.certificate.clone(),
        })
    }

    fn listen(self: &Arc<Self>, address: SocketAddr) -> io::Result<()> {
        validate_address(address)?;
        let mut listening = self.listening.lock().unwrap();
        if let Some(current) = *listening {
            if current == address {
                return Ok(());
            }
            return Err(io::Error::other(
                "Restart Agent before changing its listening address",
            ));
        }
        let listener = TcpListener::bind(address)?;
        *listening = Some(listener.local_addr()?);
        let service = self.clone();
        let active_count = Arc::new(AtomicUsize::new(0));
        std::thread::Builder::new()
            .name("device-listener".into())
            .spawn(move || {
                for socket in listener.incoming().flatten() {
                    if active_count.fetch_add(1, Ordering::Relaxed) >= 8 {
                        active_count.fetch_sub(1, Ordering::Relaxed);
                        continue;
                    }
                    let service = service.clone();
                    let active_count = active_count.clone();
                    std::thread::spawn(move || {
                        let _ = service.serve(socket);
                        active_count.fetch_sub(1, Ordering::Relaxed);
                    });
                }
            })?;
        Ok(())
    }

    fn serve(&self, mut socket: std::net::TcpStream) -> io::Result<()> {
        socket.set_read_timeout(Some(Duration::from_secs(10)))?;
        socket.set_write_timeout(Some(Duration::from_secs(10)))?;
        let mut first_byte = [0];
        socket.peek(&mut first_byte)?;
        if first_byte[0] == b'A' {
            return discovery::answer(&mut socket, &self.description()?);
        }
        let source_address = socket.peer_addr()?;
        let certificate_list = self
            .store
            .lock()
            .unwrap()
            .peer_list
            .iter()
            .filter(|peer| peer.may_control_us)
            .map(|peer| peer.device.certificate.clone())
            .collect();
        let config = transport::server_config(&self.identity, certificate_list)?;
        let connection = rustls::ServerConnection::new(config).map_err(io::Error::other)?;
        let mut stream = rustls::StreamOwned::new(connection, socket);
        let request: RemoteRequest = transport::receive_limited(&mut stream, 8 * 1024 * 1024)?;
        let peer_id = stream
            .conn
            .peer_certificates()
            .and_then(|list| list.first())
            .map(|certificate| device_id(certificate));
        let result = match request {
            RemoteRequest::RequestAccess { device } => (|| {
                if source_address.ip() != device.address.ip() {
                    return Err(io::Error::other(
                        "Pairing source does not match device address",
                    ));
                }
                tailscale::identify(source_address)?;
                let device = self.verify_device(device)?;
                self.accept_request(device)
            })(),
            RemoteRequest::Pair { ref device, .. }
                if source_address.ip() != device.address.ip() =>
            {
                Err(io::Error::other(
                    "Pairing source does not match device address",
                ))
            }
            request => self.dispatch(peer_id.as_deref(), request),
        };
        transport::send(&mut stream, &response(result))
    }

    fn description(&self) -> io::Result<discovery::Description> {
        Ok(discovery::Description {
            device: self.local_device()?,
            tool_list: vec!["shell".into(), "computer".into()],
        })
    }

    fn verify_device(&self, device: Device) -> io::Result<Device> {
        validate_address(device.address)?;
        let mut connection = transport::connect(device.address, device.certificate.clone(), None)?;
        transport::send(&mut connection, &RemoteRequest::Describe)?;
        let description: discovery::Description =
            serde_json::from_value(read_result(&mut connection)?)?;
        if description.device.id() != device.id() || description.device.address != device.address {
            return Err(io::Error::other("Pairing device identity changed"));
        }
        if description.device.name.is_empty() || description.device.name.len() > 100 {
            return Err(io::Error::other("Invalid device name"));
        }
        Ok(description.device)
    }

    fn pairing_status(&self, id: &str) -> &'static str {
        let store = self.store.lock().unwrap();
        if store
            .peer_list
            .iter()
            .any(|peer| peer.device.id() == id && peer.may_control_us)
        {
            "Connected"
        } else if store.declined_id_list.iter().any(|declined| declined == id) {
            "Declined"
        } else if self.pending_by_id.lock().unwrap().contains_key(id) {
            "Awaiting approval"
        } else {
            "Access not granted"
        }
    }

    fn accept_request(&self, device: Device) -> io::Result<Value> {
        let id = device.id();
        // Use the same lock order as approval and revocation.
        let store = self.store.lock().unwrap();
        if store.declined_id_list.contains(&id) {
            return Ok(json!({"status": "Declined"}));
        }
        if store
            .peer_list
            .iter()
            .any(|peer| peer.device.id() == id && peer.may_control_us)
        {
            return Ok(json!({"status": "Connected"}));
        }
        self.pending_by_id
            .lock()
            .unwrap()
            .entry(id.clone())
            .or_insert(device);
        Ok(json!({"status": "Awaiting approval", "pending_approval": id}))
    }

    fn dispatch(&self, peer_id: Option<&str>, request: RemoteRequest) -> io::Result<Value> {
        if let RemoteRequest::Pair { secret, device } = request {
            validate_address(device.address)?;
            if device.name.is_empty() || device.name.len() > 100 || device.certificate.len() > 16384
            {
                return Err(io::Error::other("Invalid device identity"));
            }
            let mut roots = rustls::RootCertStore::empty();
            roots
                .add(rustls::pki_types::CertificateDer::from(
                    device.certificate.clone(),
                ))
                .map_err(|_| io::Error::other("Invalid device certificate"))?;
            let mut invitation_by_secret = self.invitation_by_secret.lock().unwrap();
            let invitation = invitation_by_secret
                .get_mut(&secret)
                .filter(|invitation| invitation.created.elapsed() < Duration::from_secs(600))
                .ok_or_else(|| io::Error::other("Invitation is invalid or expired"))?;
            let id = device.id();
            if invitation
                .claimed_by
                .as_ref()
                .is_some_and(|claimed| claimed != &id)
            {
                return Err(io::Error::other("Invitation already used"));
            }
            invitation.claimed_by = Some(id.clone());
            drop(invitation_by_secret);
            let device = self.verify_device(device)?;
            return self.accept_request(device);
        }
        if matches!(request, RemoteRequest::Describe) {
            return Ok(serde_json::to_value(self.description()?)?);
        }
        if let RemoteRequest::PairingStatus { id } = request {
            return Ok(json!({"status": self.pairing_status(&id)}));
        }

        let store = self.store.lock().unwrap();
        let peer_id = peer_id.ok_or_else(|| {
            io::Error::new(io::ErrorKind::PermissionDenied, "Device is not authorized")
        })?;
        if !store
            .peer_list
            .iter()
            .any(|peer| peer.may_control_us && peer.device.id() == peer_id)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Device is not authorized",
            ));
        }
        // Hold authorization through dispatch so revocation cannot race a new submission.
        match request {
            RemoteRequest::Info => Ok(
                json!({"id": device_id(&self.identity.certificate), "name": store.name,
                "tool_list": ["shell", "computer"]}),
            ),
            RemoteRequest::Submit { id, tool } => Ok(serde_json::to_value(
                self.tools.submit(peer_id, &id, tool)?,
            )?),
            RemoteRequest::Status { id } => {
                Ok(serde_json::to_value(self.tools.get(peer_id, &id)?)?)
            }
            RemoteRequest::Cancel { id } => {
                Ok(serde_json::to_value(self.tools.cancel(peer_id, &id)?)?)
            }
            RemoteRequest::Pair { .. }
            | RemoteRequest::Describe
            | RemoteRequest::PairingStatus { .. } => unreachable!(),
            RemoteRequest::RequestAccess { .. } => Err(io::Error::other(
                "Pairing requires a verified Tailscale connection",
            )),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum RemoteRequest {
    Pair { secret: String, device: Device },
    Describe,
    RequestAccess { device: Device },
    PairingStatus { id: String },
    Info,
    Submit { id: String, tool: Tool },
    Status { id: String },
    Cancel { id: String },
}

#[derive(Serialize, Deserialize)]
struct Invitation {
    device: Device,
    secret: String,
}

struct InvitationState {
    created: Instant,
    claimed_by: Option<String>,
}

pub(super) fn response(result: io::Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"ok": true, "result": value}),
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    }
}

fn read_result(stream: &mut impl io::Read) -> io::Result<Value> {
    let response: Value = transport::receive(stream)?;
    if response["ok"] != true {
        return Err(io::Error::other(
            response["error"]
                .as_str()
                .unwrap_or("Device request failed"),
        ));
    }
    Ok(response["result"].clone())
}

fn validate_address(address: SocketAddr) -> io::Result<()> {
    let allowed = match address.ip() {
        std::net::IpAddr::V4(ip) => {
            ip.is_loopback() || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
        }
        std::net::IpAddr::V6(ip) => ip.is_loopback(),
    };
    if !allowed || address.port() == 0 {
        return Err(io::Error::other(
            "Use a Tailscale IPv4 address and a nonzero port",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(name: &str) -> Arc<DeviceService> {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(".scratch/2026-11-08-03-10-device-tools")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&directory).unwrap();
        let probe = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = probe.local_addr().unwrap();
        drop(probe);
        let service = Arc::new(DeviceService {
            identity: Identity::generate().unwrap(),
            store: Mutex::new(DeviceStore::default()),
            store_path: directory.join("devices.json"),
            invitation_by_secret: Mutex::new(HashMap::new()),
            pending_by_id: Mutex::new(HashMap::new()),
            listening: Mutex::new(None),
            candidate_by_id: Mutex::new(HashMap::new()),
            status_by_id: Mutex::new(HashMap::new()),
            connection_status: Mutex::new(String::new()),
            tools: ToolStore::load(directory).unwrap(),
        });
        service.configure(address, name.into()).unwrap();
        service
    }

    fn pair(controller: &DeviceService, target: &DeviceService) {
        let invitation = target.invite().unwrap();
        controller
            .pair(invitation["invitation"].as_str().unwrap())
            .unwrap();
        target
            .approve(&device_id(&controller.identity.certificate))
            .unwrap();
    }

    #[test]
    fn removing_one_direction_preserves_the_other() {
        let first = device("first");
        let second = device("second");
        pair(&first, &second);
        pair(&second, &first);
        let first_id = device_id(&first.identity.certificate);
        let second_id = device_id(&second.identity.certificate);

        first.revoke_access(&second_id).unwrap();
        assert!(first.call("second", RemoteRequest::Info).is_ok());
        assert!(second.call("first", RemoteRequest::Info).is_err());

        assert!(first.approve(&second_id).is_err());
        first.allow_requests(&second_id).unwrap();
        pair(&second, &first);
        first.remove_target(&second_id).unwrap();
        assert!(first.call("second", RemoteRequest::Info).is_err());
        assert!(second.call("first", RemoteRequest::Info).is_ok());
        let saved = DeviceStore::load(&first.store_path).unwrap();
        assert!(saved.peer_list[0].may_control_us);
        assert!(!saved.peer_list[0].target);
        assert!(second.store.lock().unwrap().peer(&first_id).is_ok());
    }

    #[test]
    fn declined_requests_stay_declined_after_retry_and_reload() {
        let first = device("first");
        let second = device("second");
        let invitation = second.invite().unwrap();
        let invitation = invitation["invitation"].as_str().unwrap();
        let id = device_id(&first.identity.certificate);
        first.pair(invitation).unwrap();
        first.pair(invitation).unwrap();
        assert_eq!(second.pending_by_id.lock().unwrap().len(), 1);

        second.decline(&id).unwrap();
        *second.store.lock().unwrap() = DeviceStore::load(&second.store_path).unwrap();
        assert_eq!(first.pair(invitation).unwrap()["status"], "Declined");
        assert!(second.pending_by_id.lock().unwrap().is_empty());
        assert!(first.call("second", RemoteRequest::Info).is_err());
    }

    #[test]
    fn pairing_requires_the_requesting_endpoint_to_hold_its_private_key() {
        let first = device("first");
        let impostor = device("impostor");
        let mut claimed = first.local_device().unwrap();
        claimed.address = impostor.local_device().unwrap().address;
        assert!(first.verify_device(claimed).is_err());
        assert!(
            first
                .dispatch(
                    None,
                    RemoteRequest::RequestAccess {
                        device: impostor.local_device().unwrap(),
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn pairing_is_directional_persistent_and_revocable() {
        let controller = device("controller");
        let target = device("target");
        let invitation = target.invite().unwrap();
        controller
            .pair(invitation["invitation"].as_str().unwrap())
            .unwrap();
        assert!(controller.call("target", RemoteRequest::Info).is_err());
        let controller_id = device_id(&controller.identity.certificate);
        target.approve(&controller_id).unwrap();
        assert_eq!(
            controller.call("target", RemoteRequest::Info).unwrap()["tool_list"],
            json!(["shell", "computer"])
        );
        assert!(target.call("controller", RemoteRequest::Info).is_err());
        *target.store.lock().unwrap() = DeviceStore::load(&target.store_path).unwrap();
        *controller.store.lock().unwrap() = DeviceStore::load(&controller.store_path).unwrap();
        assert!(controller.call("target", RemoteRequest::Info).is_ok());
        target.revoke_access(&controller_id).unwrap();
        assert!(controller.call("target", RemoteRequest::Info).is_err());
    }

    #[test]
    fn anonymous_wrong_identity_and_wrong_server_are_rejected() {
        let controller = device("controller");
        let target = device("target");
        pair(&controller, &target);
        let address = target.local_device().unwrap().address;
        let mut anonymous =
            transport::connect(address, target.identity.certificate.clone(), None).unwrap();
        transport::send(&mut anonymous, &RemoteRequest::Info).unwrap();
        assert!(read_result(&mut anonymous).is_err());

        let impostor = Identity::generate().unwrap();
        let mut connection = transport::connect(
            address,
            target.identity.certificate.clone(),
            Some(&impostor),
        )
        .unwrap();
        let accepted = transport::send(&mut connection, &RemoteRequest::Info)
            .and_then(|_| read_result(&mut connection));
        assert!(accepted.is_err());

        let mut wrong_server = transport::connect(
            address,
            impostor.certificate.clone(),
            Some(&controller.identity),
        )
        .unwrap();
        assert!(transport::send(&mut wrong_server, &RemoteRequest::Info).is_err());
    }

    #[test]
    fn invitations_expire_and_cannot_be_claimed_by_another_device() {
        let first = device("first");
        let second = device("second");
        let target = device("target");
        let invitation = target.invite().unwrap();
        let invitation = invitation["invitation"].as_str().unwrap();
        first.pair(invitation).unwrap();
        assert!(second.pair(invitation).is_err());
        for entry in target.invitation_by_secret.lock().unwrap().values_mut() {
            entry.created = Instant::now() - Duration::from_secs(601);
        }
        assert!(first.pair(invitation).is_err());
    }

    #[test]
    fn malformed_certificate_cannot_poison_pairing() {
        let controller = device("controller");
        let target = device("target");
        let invitation = target.invite().unwrap();
        let encoded = invitation["invitation"].as_str().unwrap();
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(encoded)
            .unwrap();
        let invitation: Invitation = serde_json::from_slice(&bytes).unwrap();
        let mut device = controller.local_device().unwrap();
        device.certificate = vec![1, 2, 3];
        assert!(
            target
                .dispatch(
                    None,
                    RemoteRequest::Pair {
                        secret: invitation.secret,
                        device
                    }
                )
                .is_err()
        );
        assert!(target.pending_by_id.lock().unwrap().is_empty());
        controller.pair(encoded).unwrap();
    }

    #[test]
    fn remote_shell_survives_disconnect_and_duplicate_submission() {
        let controller = device("controller");
        let target = device("target");
        pair(&controller, &target);
        let id = uuid::Uuid::new_v4().to_string();
        let directory = target.store_path.parent().unwrap().to_path_buf();
        let tool = Tool::Shell {
            command: "sleep 0.2; printf x >> count; printf done".into(),
            cwd: directory.clone(),
            timeout_seconds: 10,
        };
        let submit = || RemoteRequest::Submit {
            id: id.clone(),
            tool: tool.clone(),
        };
        assert_eq!(
            controller.call("target", submit()).unwrap()["status"],
            "running"
        );
        controller.call("target", submit()).unwrap();
        let started = Instant::now();
        loop {
            let job = controller
                .call("target", RemoteRequest::Status { id: id.clone() })
                .unwrap();
            if job["status"] != "running" {
                assert_eq!(job["result"]["stdout"], "done");
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(25));
        }
        controller.call("target", submit()).unwrap();
        assert_eq!(std::fs::read(directory.join("count")).unwrap(), b"x");
        let reloaded = ToolStore::load(directory).unwrap();
        let job = reloaded
            .get(&device_id(&controller.identity.certificate), &id)
            .unwrap();
        assert_eq!(job.status, "completed");
        assert!(reloaded.get("another-device", &id).is_err());
        assert!(
            controller
                .call(
                    "target",
                    RemoteRequest::Submit {
                        id,
                        tool: Tool::Shell {
                            command: "true".into(),
                            cwd: PathBuf::from("/tmp"),
                            timeout_seconds: 1
                        }
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn remote_shell_can_be_cancelled_and_revocation_blocks_new_tools() {
        let controller = device("controller");
        let target = device("target");
        pair(&controller, &target);
        let id = uuid::Uuid::new_v4().to_string();
        let tool = Tool::Shell {
            command: "sleep 30".into(),
            cwd: PathBuf::from("/tmp"),
            timeout_seconds: 60,
        };
        controller
            .call(
                "target",
                RemoteRequest::Submit {
                    id: id.clone(),
                    tool: tool.clone(),
                },
            )
            .unwrap();
        controller
            .call("target", RemoteRequest::Cancel { id: id.clone() })
            .unwrap();
        let started = Instant::now();
        loop {
            let job = controller
                .call("target", RemoteRequest::Status { id: id.clone() })
                .unwrap();
            if job["status"] != "running" {
                assert_eq!(job["status"], "cancelled");
                break;
            }
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(25));
        }
        target
            .revoke_access(&device_id(&controller.identity.certificate))
            .unwrap();
        assert!(
            controller
                .call(
                    "target",
                    RemoteRequest::Submit {
                        id: uuid::Uuid::new_v4().to_string(),
                        tool
                    }
                )
                .is_err()
        );
    }
}
