use super::central_service::CentralService;
use super::service_store::ServiceStore;
use crate::devices::{DeviceService, ToolConnection};
use crate::service::chatgpt::ChatGptClient;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;

pub(crate) fn run() -> io::Result<()> {
    let directory = crate::platform::runtime_dir()?.join("service");
    fs::create_dir_all(&directory)?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;

    let store = Arc::new(ServiceStore::load(directory.clone())?);
    let device_service = DeviceService::load_central(directory.clone())?;
    let devices = ToolConnection::new(device_service.clone());
    let client = Arc::new(ChatGptClient::new_central(&directory)?);
    let service = CentralService::new(store, client, devices);
    *device_service.service_handler.lock().unwrap() = Some(Arc::new(move |sender, request| {
        service.handle(sender, request)
    }));
    crate::devices::start_admin(device_service, &directory)?;

    loop {
        std::thread::park();
    }
}
