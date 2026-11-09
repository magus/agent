use crate::devices::Device;
use serde_json::Value;
use std::collections::BTreeMap;

pub(super) struct DeviceRow {
    pub id: String,
    pub name: String,
    pub online: bool,
    pub outgoing: bool,
    pub incoming: bool,
    pub pending: bool,
    pub declined: bool,
    pub outgoing_status: String,
}

impl DeviceRow {
    pub fn list(snapshot: &Value) -> Vec<Self> {
        let mut row_by_id = BTreeMap::new();
        for peer in snapshot["peer_list"].as_array().into_iter().flatten() {
            if let Some(mut row) = Self::from_device(&peer["device"]) {
                row.outgoing = peer["target"] == true;
                row.incoming = peer["may_control_us"] == true;
                row_by_id.insert(row.id.clone(), row);
            }
        }
        for candidate in snapshot["candidate_list"].as_array().into_iter().flatten() {
            if let Some(device) = Self::from_device(&candidate["description"]["device"]) {
                let row = row_by_id.entry(device.id.clone()).or_insert(device);
                row.online = true;
            }
        }
        for pending in snapshot["pending_list"].as_array().into_iter().flatten() {
            if let Some(device) = Self::from_device(&pending["device"]) {
                row_by_id.entry(device.id.clone()).or_insert(device).pending = true;
            }
        }
        for row in row_by_id.values_mut() {
            row.declined = snapshot["declined_id_list"]
                .as_array()
                .is_some_and(|list| list.iter().any(|id| id.as_str() == Some(&row.id)));
            row.outgoing_status = snapshot["status_by_id"][&row.id]
                .as_str()
                .unwrap_or("")
                .into();
            row.online |= row.outgoing_status == "Connected";
        }
        let mut row_list = row_by_id.into_values().collect::<Vec<_>>();
        row_list.sort_by(|first, second| first.name.cmp(&second.name));
        row_list
    }

    fn from_device(value: &Value) -> Option<Self> {
        let device: Device = serde_json::from_value(value.clone()).ok()?;
        Some(Self {
            id: device.id(),
            name: device.name,
            online: false,
            outgoing: false,
            incoming: false,
            pending: false,
            declined: false,
            outgoing_status: String::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn one_identity_keeps_both_permissions_and_pending_state() {
        let device = json!({"name":"Mini", "address":"100.122.16.74:47321", "certificate":[1,2,3]});
        let snapshot = json!({
            "peer_list":[{"device":device, "target":true, "may_control_us":true}],
            "candidate_list":[{"description":{"device":device}}, {"description":{"device":device}}],
            "pending_list":[{"device":device}]
        });
        let row_list = DeviceRow::list(&snapshot);
        assert_eq!(row_list.len(), 1);
        let row = &row_list[0];
        assert!(row.outgoing && row.incoming && row.online && row.pending);
    }

    #[test]
    fn undiscovered_known_devices_remain_visible_offline() {
        let snapshot = json!({"peer_list":[{
            "device":{"name":"Mini", "address":"100.122.16.74:47321", "certificate":[1,2,3]},
            "target":true, "may_control_us":false
        }]});
        let row_list = DeviceRow::list(&snapshot);
        assert_eq!(row_list.len(), 1);
        assert!(!row_list[0].online);
        assert!(row_list[0].outgoing);
        assert!(!row_list[0].incoming);
    }
}
