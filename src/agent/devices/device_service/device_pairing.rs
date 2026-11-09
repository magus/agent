use super::device_connection::validate_address;
use super::device_service::DeviceService;
use super::remote_request::{RemoteRequest, read_result};
use crate::devices::device_store::{Device, Peer, save_json};
use crate::devices::{discovery, tailscale, transport};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io;
use std::time::{Duration, Instant};

impl DeviceService {
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

    pub(super) fn verify_device(&self, device: Device) -> io::Result<Device> {
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

    pub(super) fn pairing_status(&self, id: &str) -> &'static str {
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

    pub(super) fn accept_request(&self, device: Device) -> io::Result<Value> {
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
}

#[derive(Serialize, Deserialize)]
pub(super) struct Invitation {
    pub(super) device: Device,
    pub(super) secret: String,
}

pub(super) struct InvitationState {
    pub(super) created: Instant,
    pub(super) claimed_by: Option<String>,
}
