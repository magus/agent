use gpui_kit::*;

pub(crate) fn activate(handle: Option<AnyWindowHandle>, cx: &mut App) -> bool {
    let activated = handle.is_some_and(|handle| {
        handle
            .update(cx, |_, window, _| {
                window.activate_window();
                window.refresh();
                request_redraw(window);
            })
            .is_ok()
    });
    if activated {
        cx.activate(true);
    }
    activated
}

pub(crate) fn observe<V: Render>(handle: AnyWindowHandle, view: &Entity<V>, cx: &mut App) {
    cx.observe(view, move |_, cx| {
        let _ = handle.update(cx, |_, window, _| {
            window.refresh();
            request_redraw(window);
        });
    })
    .detach();
}

fn request_redraw(window: &Window) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return;
    };
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else {
        return;
    };

    // Schedule display outside GPUI's update so the callback can borrow the app.
    // The window owns this view, and observers run on the main thread.
    unsafe {
        let view = handle.ns_view.as_ptr().cast::<objc2::runtime::AnyObject>();
        let layer: *mut objc2::runtime::AnyObject = objc2::msg_send![view, layer];
        let _: () = objc2::msg_send![layer,
            performSelector: objc2::sel!(display),
            withObject: std::ptr::null::<objc2::runtime::AnyObject>(),
            afterDelay: 0.0_f64
        ];
    }
}
