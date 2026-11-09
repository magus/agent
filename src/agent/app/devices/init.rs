use super::devices_window::DevicesWindow;
use super::menu_item_list::OpenDevices;
use crate::app::args::Args;
use crate::devices::DeviceService;
use std::io;

pub(crate) fn init(
    args: Args,
    cx: &mut gpui_kit::App,
) -> io::Result<std::sync::Arc<crate::devices::ToolConnection>> {
    let runtime_dir = crate::platform::runtime_dir()?;
    let service = DeviceService::load(runtime_dir.clone())?;
    crate::devices::start_admin(service.clone(), &runtime_dir)?;
    let window_service = service.clone();
    cx.on_action(move |_: &OpenDevices, cx| DevicesWindow::open(window_service.clone(), cx));
    if args == Args::Devices {
        DevicesWindow::open(service.clone(), cx);
    }
    Ok(crate::devices::ToolConnection::new(service))
}
