use super::DeviceService;
use crate::devices::device_store::Device;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Serialize)]
pub(crate) struct DeviceView {
    name: String,
    execution_device: bool,
    device_list: Vec<DeviceConnection>,
}

impl DeviceService {
    pub(crate) fn view(&self) -> DeviceView {
        let store = self.store.lock().unwrap().clone();
        let mut connection_by_id = BTreeMap::new();

        for peer in store.peer_list {
            let mut connection = DeviceConnection::new(&peer.device);
            connection.outgoing = peer.target;
            connection.incoming = peer.may_control_us;
            connection_by_id.insert(connection.id.clone(), connection);
        }

        for candidate in self.candidate_by_id.lock().unwrap().values() {
            let device = &candidate.description.device;
            let connection = connection_by_id
                .entry(device.id())
                .or_insert_with(|| DeviceConnection::new(device));
            connection.discovered = true;
        }

        for device in self.pending_by_id.lock().unwrap().values() {
            connection_by_id
                .entry(device.id())
                .or_insert_with(|| DeviceConnection::new(device))
                .pending = true;
        }

        let status_by_id = self.status_by_id.lock().unwrap();
        for connection in connection_by_id.values_mut() {
            connection.status = status_by_id
                .get(&connection.id)
                .cloned()
                .unwrap_or_default();
            connection.blocked = store.declined_id_list.contains(&connection.id);
        }
        let mut device_list = connection_by_id.into_values().collect::<Vec<_>>();
        device_list.sort_by(|first, second| first.name.cmp(&second.name));

        DeviceView {
            name: store.name,
            execution_device: self.execution_device,
            device_list,
        }
    }
}

#[derive(Serialize)]
struct DeviceConnection {
    id: String,
    name: String,
    address: String,
    outgoing: bool,
    incoming: bool,
    pending: bool,
    blocked: bool,
    discovered: bool,
    status: String,
}

impl DeviceConnection {
    fn new(device: &Device) -> Self {
        Self {
            id: device.id(),
            name: device.name.clone(),
            address: device.address.to_string(),
            outgoing: false,
            incoming: false,
            pending: false,
            blocked: false,
            discovered: false,
            status: String::new(),
        }
    }
}
