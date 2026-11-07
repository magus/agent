#[path = "DesktopService.rs"]
mod desktop_service;
#[path = "PermissionSetup.rs"]
mod permission_setup;

use desktop_service::DesktopService;
use gpui_kit::component::Theme;
use gpui_kit::*;
use permission_setup::PermissionSetup;
use std::io::{self, Write};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

gpui_kit::actions!(agent, [Quit, Setup]);
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main() {
    if let Err(error) = run() {
        eprintln!("Agent: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let argument_list: Vec<String> = std::env::args().skip(1).collect();
    let diagnostics = match argument_list.as_slice() {
        [] => false,
        [argument] if argument == "--setup" => false,
        [argument] if argument == "--permissions" => true,
        [argument] if argument == "--help" || argument == "-h" => {
            println!("Usage: Agent [--setup | --permissions]");
            return Ok(());
        }
        _ => return Err(io::Error::other("Unsupported arguments; use --help")),
    };

    let service = Arc::new(DesktopService::new()?);
    if diagnostics {
        let response = service.call("--permissions")?;
        let mut log = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(service.runtime_dir.join("service.log"))?;
        writeln!(log, "{response}")?;
        return Ok(());
    }
    service.start()?;

    // Signal handlers only set a flag. The app stops and reaps the helper.
    unsafe {
        signal(2, stop as *const () as usize);
        signal(15, stop as *const () as usize);
    }

    let app = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
    let reopen_service = service.clone();
    app.on_reopen(move |cx| show_setup(reopen_service.clone(), cx));
    app.run(move |cx| {
        gpui_kit::init(cx);
        Theme::sync_system_appearance(None, cx);
        cx.on_action(|_: &Quit, cx| cx.quit());
        let setup_service = service.clone();
        cx.on_action(move |_: &Setup, cx| show_setup(setup_service.clone(), cx));
        cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
        cx.set_menus([Menu::new("Agent").items([
            MenuItem::action("Set up Agent", Setup),
            MenuItem::separator(),
            MenuItem::action("Quit Agent", Quit),
        ])]);
        let quit_service = service.clone();
        cx.on_app_quit(move |_| {
            quit_service.stop();
            async {}
        })
        .detach();

        show_setup(service.clone(), cx);
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(500))
                    .await;
                let stopped = service.stopped().unwrap_or(true);
                let stop_requested = STOP_REQUESTED.load(Ordering::Relaxed)
                    || service.runtime_dir.join("stop").exists();
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
    });
    Ok(())
}

fn show_setup(service: Arc<DesktopService>, cx: &mut App) {
    if let Some(handle) = cx.windows().first() {
        let _ = handle.update(cx, |_, window, _| window.activate_window());
        cx.activate(true);
        return;
    }
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(560.), px(430.)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some("Agent".into()),
            ..Default::default()
        }),
        is_resizable: false,
        ..Default::default()
    };
    let result = gpui_kit::open_window(options, cx, |_, cx| {
        cx.new(|cx| PermissionSetup::new(service, cx))
    });
    match result {
        Ok((handle, view)) => {
            cx.observe(&view, move |_, cx| {
                let _ = handle.update(cx, |_, window, _| {
                    window.refresh();
                    request_redraw(window);
                });
            })
            .detach();
        }
        Err(error) => {
            eprintln!("Could not open Agent setup. {error}");
            cx.quit();
        }
    }
    cx.activate(true);
}

fn request_redraw(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    // GPUI's display callback can be idle when an async setup request finishes.
    // The live window owns this NSView, and GPUI invokes us on the main thread.
    unsafe {
        let view = handle.ns_view.as_ptr().cast::<objc2::runtime::AnyObject>();
        let layer: *mut objc2::runtime::AnyObject = objc2::msg_send![view, layer];
        let _: () = objc2::msg_send![layer, setNeedsDisplay];
    }
}

extern "C" fn stop(_: i32) {
    STOP_REQUESTED.store(true, Ordering::Relaxed);
}

unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
}
