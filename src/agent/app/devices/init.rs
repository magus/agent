use super::device_service::DeviceService;
use super::devices_window::DevicesWindow;
use super::menu_item_list::OpenDevices;
use crate::app::args::Args;
use std::io;

pub(crate) fn init(args: Args, cx: &mut gpui_kit::App) -> io::Result<()> {
    let runtime_dir = crate::app::core::runtime_dir()?;
    let service = DeviceService::load(runtime_dir.clone())?;
    super::local_client::start(service.clone(), &runtime_dir)?;
    let window_service = service.clone();
    cx.on_action(move |_: &OpenDevices, cx| DevicesWindow::open(window_service.clone(), cx));
    if args == Args::Devices {
        DevicesWindow::open(service, cx);
    }
    Ok(())
}
