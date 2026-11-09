use super::device_service::DeviceService;
use super::remote_request::{RemoteRequest, read_result};
use crate::devices::device_store::save_json;
use crate::devices::identity::device_id;
use crate::devices::system_name::system_name;
use crate::devices::{discovery, tailscale, transport};
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;

impl DeviceService {
    pub fn find_device(&self, host: &str) -> io::Result<Value> {
        let host = host.trim();
        if host.is_empty() || host.contains('/') || host.len() > 253 {
            return Err(io::Error::other("Enter an IP address or hostname"));
        }
        let (hostname, port) = match host.rsplit_once(':') {
            Some((hostname, port)) => {
                let port = port
                    .parse::<u16>()
                    .map_err(|_| io::Error::other("Invalid port"))?;
                if port == 0 || hostname.is_empty() {
                    return Err(io::Error::other("Enter a hostname and a nonzero port"));
                }
                (hostname, port)
            }
            None => (host, 47321),
        };
        let address_list = match (hostname, port).to_socket_addrs() {
            Ok(address_list) => address_list.collect::<Vec<_>>(),
            Err(_) => {
                let mut address = tailscale::resolve_name(hostname)?;
                address.set_port(port);
                vec![address]
            }
        };
        let mut last_error = io::Error::other(
            "Could not verify this address. Use Pair manually with an invitation.",
        );
        for address in address_list.into_iter().filter(SocketAddr::is_ipv4) {
            let result = (|| {
                let node_id = tailscale::identify(address)?;
                let candidate = Self::discover_device(address, node_id)?;
                if candidate.description.device.id() == device_id(&self.identity.certificate) {
                    return Err(io::Error::other("This address belongs to this device"));
                }
                let description = serde_json::to_value(&candidate.description)?;
                self.candidate_by_id
                    .lock()
                    .unwrap()
                    .insert(candidate.description.device.id(), candidate);
                Ok(description)
            })();
            match result {
                Ok(description) => return Ok(description),
                Err(error) => last_error = error,
            }
        }
        Err(io::Error::other(format!(
            "Could not verify device. {last_error}. Use Pair manually with an invitation."
        )))
    }

    fn discover_device(address: SocketAddr, node_id: String) -> io::Result<discovery::Candidate> {
        let description = discovery::query(address, &node_id)?;
        let mut connection =
            transport::connect(address, description.device.certificate.clone(), None)?;
        transport::send(&mut connection, &RemoteRequest::Describe)?;
        let verified: discovery::Description =
            serde_json::from_value(read_result(&mut connection)?)?;
        if verified.device.id() != description.device.id() || verified.device.address != address {
            return Err(io::Error::other("Agent identity changed during discovery"));
        }
        Ok(discovery::Candidate {
            description: verified,
            node_id,
        })
    }

    pub fn discover(self: &Arc<Self>) -> io::Result<Value> {
        let system_name = system_name()?;
        {
            let mut store = self.store.lock().unwrap();
            if store.name_override.is_none() && store.name != system_name {
                let mut updated = store.clone();
                updated.name = system_name;
                save_json(&self.store_path, &updated)?;
                *store = updated;
            }
        }
        let status = tailscale::status()?;
        let mut address = status.this_device.address()?;
        address.set_port(self.port);
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
                Self::discover_device(address, node.id)
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

    pub(super) fn refresh_target_status(&self) {
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
}
