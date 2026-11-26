mod computer_permissions;
mod device_service;
mod device_store;
mod discovery;
mod identity;
#[cfg(not(target_os = "ios"))]
mod local_client;
mod system_name;
mod tailscale;
mod tool_connection;
mod tool_store;
mod transport;

pub(crate) use device_service::DeviceService;
#[cfg(not(target_os = "ios"))]
pub(crate) use local_client::{run as run_client, run_at as run_client_at, start as start_admin};
pub(crate) use tool_connection::ToolConnection;

#[cfg(test)]
pub(crate) use device_service::device_with_kind;
