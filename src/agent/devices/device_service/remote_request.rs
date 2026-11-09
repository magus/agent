use crate::devices::device_store::Device;
use crate::devices::tool_store::Tool;
use crate::devices::transport;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::io;

#[derive(Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(in crate::devices) enum RemoteRequest {
    Service { request: Value },
    Pair { secret: String, device: Device },
    Describe,
    RequestAccess { device: Device },
    PairingStatus { id: String },
    Info,
    Submit { id: String, tool: Tool },
    Status { id: String },
    Cancel { id: String },
}

pub(in crate::devices) fn response(result: io::Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"ok": true, "result": value}),
        Err(error) => json!({"ok": false, "error": error.to_string()}),
    }
}

pub(super) fn read_result(stream: &mut impl io::Read) -> io::Result<Value> {
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
