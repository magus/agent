use super::menu_item_list::OpenDesktopPermissions;
use super::permission_setup::PermissionSetup;
use super::service::DesktopService;
use crate::app::args::Args;
use gpui_kit::App;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

pub(crate) fn init(args: Args, cx: &mut App) -> io::Result<()> {
    let service = Arc::new(DesktopService::new()?);
    service.start()?;

    // Signal handlers only set a flag. The app stops and reaps the helper.
    unsafe {
        signal(2, stop as *const () as usize);
        signal(15, stop as *const () as usize);
    }

    let setup_service = service.clone();
    cx.on_action(move |_: &OpenDesktopPermissions, cx| {
        PermissionSetup::open(setup_service.clone(), cx)
    });

    let quit_service = service.clone();
    cx.on_app_quit(move |_| {
        quit_service.stop();
        async {}
    })
    .detach();

    if args == Args::Setup {
        PermissionSetup::open(service.clone(), cx);
    }

    cx.spawn(async move |cx| {
        loop {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            let stopped = service.stopped().unwrap_or(true);
            let stop_requested =
                STOP_REQUESTED.load(Ordering::Relaxed) || service.runtime_dir.join("stop").exists();
            if stopped || stop_requested {
                if stopped {
                    eprintln!("Agent desktop service exited. See service.log.");
                }
                cx.update(|cx| cx.quit());
                break;
            }
        }
    })
    .detach();
    Ok(())
}

extern "C" fn stop(_: i32) {
    STOP_REQUESTED.store(true, Ordering::Relaxed);
}

unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
}

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);
