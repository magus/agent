use crate::desktop_service::{DesktopService, Permissions};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::{prelude::*, *};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(crate) struct PermissionSetup {
    service: Arc<DesktopService>,
    permissions: Option<Permissions>,
    busy: bool,
    message: Option<String>,
    permission_error: Option<String>,
    preview: Option<CapturePreview>,
    confirmed: bool,
    saved_confirmation: bool,
    _poll_task: Task<()>,
}

impl PermissionSetup {
    pub(crate) fn new(service: Arc<DesktopService>, cx: &mut Context<Self>) -> Self {
        let saved_confirmation = read_confirmation(&service.runtime_dir);
        let poll_service = service.clone();
        let poll_task = cx.spawn(async move |view, cx| {
            loop {
                let service = poll_service.clone();
                let result = cx
                    .background_executor()
                    .spawn(async move { service.permissions() })
                    .await;
                let updated = view.update(cx, |view, cx| {
                    match result {
                        Ok(permissions) => {
                            if view.permissions.as_ref() == Some(&permissions) {
                                return;
                            }
                            if !permissions.granted() {
                                view.clear_preview();
                                view.confirmed = false;
                            }
                            view.permissions = Some(permissions);
                            view.permission_error = None;
                        }
                        Err(error) => {
                            view.permissions = None;
                            view.confirmed = false;
                            view.clear_preview();
                            view.permission_error = Some(error.to_string());
                        }
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(2)).await;
            }
        });
        Self {
            service,
            permissions: None,
            busy: false,
            message: None,
            permission_error: None,
            preview: None,
            confirmed: false,
            saved_confirmation,
            _poll_task: poll_task,
        }
    }

    fn request(&mut self, mode: &'static str, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.message = None;
        if mode == "--setup-capture" {
            self.clear_preview();
            self.confirmed = false;
        }
        cx.notify();

        let service = self.service.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { service.call(mode) })
                .await;

            // Own the image before updating the weak view so closing setup cannot leak it.
            let preview = if mode == "--setup-capture" {
                result.as_ref().ok().and_then(|response| {
                    response["capture"]["path"]
                        .as_str()
                        .map(|path| CapturePreview {
                            path: PathBuf::from(path),
                            captured_at: Instant::now(),
                        })
                })
            } else {
                None
            };

            let _ = view.update(cx, move |view, cx| {
                view.busy = false;
                match result {
                    Ok(_) if mode == "--setup-capture" => {
                        if preview.is_some() {
                            view.preview = preview;
                        } else {
                            view.message =
                                Some("The capture did not return an image. Try again.".into());
                        }
                    }
                    Ok(_) => {}
                    Err(error) => view.message = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn confirm(&mut self, cx: &mut Context<Self>) {
        let granted = self.permissions.as_ref().is_some_and(Permissions::granted);
        let fresh = self
            .preview
            .as_ref()
            .is_some_and(|preview| preview.captured_at.elapsed() < Duration::from_secs(300));
        if !granted || !fresh || self.busy {
            self.message = Some("Check capture again before confirming.".into());
            cx.notify();
            return;
        }

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let record = serde_json::json!({"version": 1, "confirmed_at": timestamp});
        let path = self.service.runtime_dir.join("capture-confirmation.json");
        let result = std::fs::write(&path, record.to_string());
        match result {
            Ok(()) => {
                self.message = None;
                self.confirmed = true;
                self.saved_confirmation = true;
                self.clear_preview();
            }
            Err(error) => self.message = Some(format!("Could not save setup. {error}")),
        }
        cx.notify();
    }

    fn clear_preview(&mut self) {
        self.preview = None;
    }

    fn permission_row(
        &self,
        title: &'static str,
        detail: &'static str,
        granted: Option<bool>,
        mode: &'static str,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .py_4()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(detail),
                    ),
            )
            .child(match granted {
                Some(true) => div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Enabled")
                    .into_any_element(),
                Some(false) => Button::new(mode)
                    .label("Open Settings")
                    .accessibility_label(format!("Open {title} settings"))
                    .disabled(self.busy)
                    .on_click(cx.listener(move |view, _, _, cx| view.request(mode, cx)))
                    .into_any_element(),
                None => div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.permission_error.is_some() {
                        "Unavailable"
                    } else {
                        "Checking…"
                    })
                    .into_any_element(),
            })
    }
}

impl Render for PermissionSetup {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let has_preview = self.preview.is_some();
        let height = if has_preview { 680. } else { 430. };
        if window.viewport_size().height != px(height) {
            window.resize(size(px(560.), px(height)));
        }
        let granted = self.permissions.as_ref().is_some_and(Permissions::granted);
        let accessibility = self.permissions.as_ref().map(|state| state.accessibility);
        let recording = self
            .permissions
            .as_ref()
            .map(|state| state.screen_recording);

        let heading = div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_xl()
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Set up Agent"),
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child("Allow Agent to see and control your desktop."),
            );

        let permission_list = div()
            .flex()
            .flex_col()
            .border_y_1()
            .border_color(cx.theme().border)
            .child(self.permission_row(
                "Accessibility",
                "Mouse and keyboard input",
                accessibility,
                "--open-accessibility",
                cx,
            ))
            .child(self.permission_row(
                "Screen Recording",
                "Screenshots of your desktop",
                recording,
                "--open-screen-recording",
                cx,
            ));

        let capture_status = if self.confirmed {
            "Capture confirmed"
        } else {
            "Check a real screenshot before unattended use."
        };
        let capture_action = div()
            .flex()
            .items_center()
            .justify_between()
            .gap_4()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(capture_status),
            )
            .child(
                Button::new("check-capture")
                    .label("Check capture")
                    .primary()
                    .disabled(!granted || self.busy)
                    .loading(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| view.request("--setup-capture", cx))),
            );

        let message = self
            .permission_error
            .as_ref()
            .or(self.message.as_ref())
            .cloned();
        let show_note = !self.confirmed && message.is_none() && self.preview.is_none();

        div()
            .id("permission-setup")
            .size_full().overflow_y_scroll()
            .bg(cx.theme().background).text_color(cx.theme().foreground)
            .p_6().flex().flex_col().gap_4()
            .child(heading)
            .child(permission_list)
            .child(capture_action)
            .when_some(self.preview.as_ref(), |view, preview| {
                let confirm = Button::new("confirm-capture")
                    .label("Yes, I can see them").primary().disabled(self.busy || !granted)
                    .on_click(cx.listener(|view, _, _, cx| view.confirm(cx)));
                let retry = Button::new("reject-capture")
                    .label("No, try again").disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.clear_preview();
                        view.message = Some("Check System Settings and choose Allow if macOS asks, then try capture again.".into());
                        cx.notify();
                    }));

                view.child(img(preview.path.clone()).w_full().h(px(220.)).flex_shrink_0()
                        .object_fit(ObjectFit::Contain))
                    .child(div().text_sm().child("Does this image show your other app windows?"))
                    .child(div().flex().gap_2().child(confirm).child(retry))
            })
            .when_some(message, |view, message| {
                view.child(div().text_sm().text_color(cx.theme().danger).child(message))
            })
            .when(self.confirmed, |view| {
                view.child(Button::new("run-background").label("Run in background")
                    .on_click(|_, window, _| window.remove_window()))
            })
            .when(show_note, |view| {
                let note = if self.saved_confirmation {
                    "Setup was confirmed before. macOS may ask for capture approval again."
                } else {
                    "Choose Allow if macOS asks during capture. If it requests a restart, quit and reopen Agent."
                };
                view.child(div().text_xs().text_color(cx.theme().muted_foreground).child(note))
            })
    }
}

struct CapturePreview {
    path: PathBuf,
    captured_at: Instant,
}

impl Drop for CapturePreview {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.path);
    }
}

fn read_confirmation(runtime_dir: &std::path::Path) -> bool {
    let path = runtime_dir.join("capture-confirmation.json");
    let Ok(data) = std::fs::read(path) else {
        return false;
    };
    let Ok(record) = serde_json::from_slice::<serde_json::Value>(&data) else {
        return false;
    };
    record["version"] == 1 && record["confirmed_at"].as_u64().is_some()
}
