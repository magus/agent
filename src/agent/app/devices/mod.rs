mod device_service;
mod device_store;
mod devices_window;
mod discovery;
mod identity;
mod init;
mod local_client;
mod menu_item_list;
mod tailscale;
mod tool_store;
mod transport;

pub(crate) use init::init;
pub(crate) use local_client::run as run_client;
pub(crate) use menu_item_list::menu_item_list;
