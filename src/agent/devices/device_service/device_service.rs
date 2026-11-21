use super::device_pairing::InvitationState;
use crate::devices::device_store::{Device, DeviceStore, save_json};
use crate::devices::discovery;
use crate::devices::identity::{Identity, device_id};
use crate::devices::system_name::system_name;
use crate::devices::tool_store::ToolStore;
use crate::protocol::Target;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct DeviceService {
    pub execution_device: bool,
    pub(super) port: u16,
    pub(super) identity: Identity,
    pub(super) store: Mutex<DeviceStore>,
    pub(super) store_path: PathBuf,
    pub(super) invitation_by_secret: Mutex<HashMap<String, InvitationState>>,
    pub(super) pending_by_id: Mutex<HashMap<String, Device>>,
    pub(super) listening: Mutex<Option<SocketAddr>>,
    pub(super) candidate_by_id: Mutex<HashMap<String, discovery::Candidate>>,
    pub(super) status_by_id: Mutex<HashMap<String, String>>,
    pub(super) connection_status: Mutex<String>,
    pub(in crate::devices) tools: Arc<ToolStore>,
    pub service_handler: Mutex<Option<Arc<ServiceHandler>>>,
}

impl DeviceService {
    #[cfg(not(target_os = "ios"))]
    pub fn load(runtime_dir: PathBuf) -> io::Result<Arc<Self>> {
        Self::load_with(runtime_dir, Identity::load()?, 47321, true, None)
    }

    #[cfg(not(target_os = "ios"))]
    pub fn load_central(runtime_dir: PathBuf) -> io::Result<Arc<Self>> {
        Self::load_with(
            runtime_dir,
            Identity::load_named("com.iamnoah.agent.service")?,
            47322,
            false,
            None,
        )
    }

    #[cfg(target_os = "ios")]
    pub fn load_client(runtime_dir: PathBuf, name: String) -> io::Result<Arc<Self>> {
        Self::load_with(runtime_dir, Identity::load()?, 47321, false, Some(name))
    }

    fn load_with(
        runtime_dir: PathBuf,
        identity: Identity,
        port: u16,
        execution_device: bool,
        client_name: Option<String>,
    ) -> io::Result<Arc<Self>> {
        let store_path = runtime_dir.join("devices.json");
        let mut store = DeviceStore::load(&store_path)?;
        if !store.name_migrated {
            if !store.name.is_empty() {
                store.name_override = Some(store.name.clone());
            }
            store.name_migrated = true;
        }
        if store.name_override.is_none() {
            store.name = match client_name {
                Some(name) => name,
                None => system_name()?,
            };
        }
        save_json(&store_path, &store)?;
        let service = Arc::new(Self {
            execution_device,
            port,
            identity,
            store: Mutex::new(store),
            store_path,
            invitation_by_secret: Mutex::new(HashMap::new()),
            pending_by_id: Mutex::new(HashMap::new()),
            listening: Mutex::new(None),
            candidate_by_id: Mutex::new(HashMap::new()),
            status_by_id: Mutex::new(HashMap::new()),
            connection_status: Mutex::new("Finding Tailscale".into()),
            tools: ToolStore::load(runtime_dir)?,
            service_handler: Mutex::new(None),
        });
        #[cfg(not(target_os = "ios"))]
        let background = service.clone();
        #[cfg(not(target_os = "ios"))]
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

    pub(crate) fn local_target(&self) -> Target {
        Target {
            id: device_id(&self.identity.certificate),
            name: self.store.lock().unwrap().name.clone(),
        }
    }

    pub(crate) fn target_list(&self) -> Vec<Target> {
        let mut target_list = Vec::new();
        if self.execution_device {
            target_list.push(self.local_target());
        }

        let store = self.store.lock().unwrap();
        for peer in &store.peer_list {
            if peer.target {
                target_list.push(Target {
                    id: peer.device.id(),
                    name: peer.device.name.clone(),
                });
            }
        }
        target_list
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
        json!({"id": device_id(&self.identity.certificate), "name": store.name, "name_override": store.name_override,
            "address": *self.listening.lock().unwrap(), "configured_address": store.address,
            "peer_list": store.peer_list,
            "declined_id_list": store.declined_id_list,
            "candidate_list": candidate_list,
            "status_by_id": *self.status_by_id.lock().unwrap(),
            "connection_status": *self.connection_status.lock().unwrap(),
            "pending_list": pending_list,
            "tool_list": self.tool_list()})
    }

    pub fn rename(&self, name: String) -> io::Result<Value> {
        let name = name.trim();
        if name.is_empty() || name.len() > 100 {
            return Err(io::Error::other("Use a device name with 1–100 characters"));
        }
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        updated.name = name.into();
        updated.name_override = Some(name.into());
        updated.name_migrated = true;
        save_json(&self.store_path, &updated)?;
        *store = updated;
        Ok(json!({"renamed": name}))
    }

    #[cfg(not(target_os = "ios"))]
    pub fn use_system_name(&self) -> io::Result<Value> {
        let name = system_name()?;
        let mut store = self.store.lock().unwrap();
        let mut updated = store.clone();
        updated.name = name.clone();
        updated.name_override = None;
        updated.name_migrated = true;
        save_json(&self.store_path, &updated)?;
        *store = updated;
        Ok(json!({"name": name}))
    }

    #[cfg(not(target_os = "ios"))]
    pub fn configure(self: &Arc<Self>, address: SocketAddr, name: String) -> io::Result<Value> {
        if name.trim().is_empty() || name.len() > 100 {
            return Err(io::Error::other(
                "Device name must contain 1–100 characters",
            ));
        }
        self.listen(address)?;
        let mut store = self.store.lock().unwrap();
        store.name_override = Some(name.clone());
        store.name_migrated = true;
        store.name = name;
        store.address = Some(address);
        save_json(&self.store_path, &*store)?;
        drop(store);
        Ok(self.snapshot())
    }

    pub(super) fn local_device(&self) -> io::Result<Device> {
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

    pub(in crate::devices) fn tool_list(&self) -> Vec<String> {
        if self.execution_device {
            let mut tool_list = vec!["shell".into()];
            if self.tools.computer_available() {
                tool_list.push("computer".into());
            }
            tool_list
        } else if self.port == 47322 {
            vec!["conversations".into()]
        } else {
            Vec::new()
        }
    }

    pub(super) fn description(&self) -> io::Result<discovery::Description> {
        Ok(discovery::Description {
            device: self.local_device()?,
            tool_list: self.tool_list(),
        })
    }
}

pub(crate) type ServiceHandler = dyn Fn(Target, Value) -> io::Result<Value> + Send + Sync;
