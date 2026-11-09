use super::device_row::DeviceRow;
use crate::devices::DeviceService;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Disableable, IconName};
use gpui_kit::{prelude::*, *};
use serde_json::Value;
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub(super) struct DevicesWindow {
    service: Arc<DeviceService>,
    name: Entity<InputState>,
    invitation: Entity<InputState>,
    host: Entity<InputState>,
    show_add: bool,
    scanning: bool,
    snapshot: Value,
    message_by_action: std::collections::HashMap<String, String>,
    show_manual: bool,
    show_details: bool,
    busy: bool,
}

impl DevicesWindow {
    pub fn open(service: Arc<DeviceService>, cx: &mut App) {
        if crate::app::core::window::activate(cx.default_global::<OpenWindow>().0, cx) {
            return;
        }
        let options = WindowOptions {
            window_min_size: Some(size(px(760.), px(440.))),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(900.), px(560.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Agent · Devices".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        match gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx: &mut Context<Self>| {
                let snapshot = service.snapshot();
                let name = snapshot["name"].as_str().unwrap_or("").to_string();
                cx.spawn(async move |view, cx| {
                    loop {
                        cx.background_executor().timer(Duration::from_secs(1)).await;
                        if view
                            .update(cx, |view, cx| {
                                let snapshot = view.service.snapshot();
                                if snapshot != view.snapshot {
                                    view.snapshot = snapshot;
                                    cx.notify();
                                }
                            })
                            .is_err()
                        {
                            break;
                        }
                    }
                })
                .detach();
                Self {
                    service,
                    name: cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("Device name")
                            .default_value(name)
                    }),
                    invitation: cx.new(|cx| {
                        InputState::new(window, cx)
                            .placeholder("Paste an invitation from the device you want to control")
                    }),
                    host: cx.new(|cx| {
                        InputState::new(window, cx).placeholder("IP address or hostname")
                    }),
                    show_add: false,
                    scanning: false,
                    snapshot,
                    message_by_action: std::collections::HashMap::new(),
                    show_manual: false,
                    show_details: false,
                    busy: false,
                }
            })
        }) {
            Ok((handle, view)) => {
                cx.global_mut::<OpenWindow>().0 = Some(handle);
                crate::app::core::window::observe(handle, &view, cx);
            }
            Err(error) => eprintln!("Could not open Devices: {error}"),
        }
        cx.activate(true);
    }

    fn run(
        &mut self,
        action_id: String,
        action: impl FnOnce(Arc<DeviceService>) -> io::Result<String> + Send + 'static,
        cx: &mut Context<Self>,
    ) {
        if self.busy {
            return;
        }
        self.busy = true;
        self.scanning = action_id == "refresh";
        if self.scanning {
            self.message_by_action.remove(&action_id);
        } else {
            self.message_by_action
                .insert(action_id.clone(), "Working…".into());
        }
        let service = self.service.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { action(service) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
                view.scanning = false;
                view.message_by_action
                    .insert(action_id, result.unwrap_or_else(|error| error.to_string()));
                view.snapshot = view.service.snapshot();
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl DevicesWindow {
    fn feedback(&self, id: &str) -> impl IntoElement {
        div()
            .text_sm()
            .children(self.message_by_action.get(id).cloned())
    }

    fn device_button(
        &self,
        id: &str,
        action: DeviceAction,
        label: &'static str,
        cx: &mut Context<Self>,
    ) -> Button {
        let id = id.to_string();
        Button::new(SharedString::from(format!("{action:?}-{id}")))
            .label(label)
            .disabled(self.busy)
            .on_click(cx.listener(move |view, _, _, cx| {
                let id = id.clone();
                view.run(id.clone(), move |service| action.run(&service, &id), cx);
            }))
    }

    fn device_row(&self, row: &DeviceRow, cx: &mut Context<Self>) -> AnyElement {
        let status = if row.outgoing_status == "Connected" {
            "Connected"
        } else if row.online {
            "Online"
        } else {
            "Offline"
        };
        let outgoing_status = match row.outgoing_status.as_str() {
            "Connected" => "Allowed",
            "Awaiting approval" => "Pending",
            "Declined" => "Declined",
            "Access not granted" => "Not allowed",
            _ => "",
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_3()
            .py_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(div().truncate().child(row.name.clone()))
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(status),
                            ),
                    )
                    .child(
                        div()
                            .w(px(180.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(row.outgoing, |cell| {
                                cell.child(div().text_sm().child(outgoing_status)).child(
                                    self.device_button(&row.id, DeviceAction::Remove, "Remove", cx),
                                )
                            })
                            .when(!row.outgoing, |cell| {
                                cell.child(
                                    self.device_button(
                                        &row.id,
                                        DeviceAction::Request,
                                        "Request access",
                                        cx,
                                    )
                                    .disabled(self.busy || !row.online),
                                )
                            }),
                    )
                    .child(
                        div()
                            .w(px(220.))
                            .flex_shrink_0()
                            .flex()
                            .items_center()
                            .gap_2()
                            .when(row.pending, |cell| {
                                cell.child(self.device_button(
                                    &row.id,
                                    DeviceAction::Allow,
                                    "Allow",
                                    cx,
                                ))
                                .child(self.device_button(
                                    &row.id,
                                    DeviceAction::Decline,
                                    "Decline",
                                    cx,
                                ))
                            })
                            .when(!row.pending && row.incoming, |cell| {
                                cell.child(div().text_sm().child("Allowed")).child(
                                    self.device_button(&row.id, DeviceAction::Revoke, "Revoke", cx),
                                )
                            })
                            .when(!row.pending && !row.incoming && row.declined, |cell| {
                                cell.child(div().text_sm().child("Blocked")).child(
                                    self.device_button(
                                        &row.id,
                                        DeviceAction::Retry,
                                        "Allow requests",
                                        cx,
                                    ),
                                )
                            })
                            .when(!row.pending && !row.incoming && !row.declined, |cell| {
                                cell.child(div().text_sm().child("Not allowed"))
                            }),
                    ),
            )
            .child(self.feedback(&row.id))
            .into_any_element()
    }

    fn manual_panel(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .p_3()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md()
            .child("Let another device request access")
            .child(
                Button::new("copy-invitation")
                    .label("Copy invitation")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| {
                        let message = match view.service.invite() {
                            Ok(value) => {
                                cx.write_to_clipboard(ClipboardItem::new_string(
                                    value["invitation"].as_str().unwrap().into(),
                                ));
                                "Copied. Expires in 10 minutes. Approval is still required.".into()
                            }
                            Err(error) => error.to_string(),
                        };
                        view.message_by_action.insert("copy".into(), message);
                        cx.notify();
                    })),
            )
            .child(self.feedback("copy"))
            .child("Or paste an invitation from another device")
            .child(Input::new(&self.invitation))
            .child(
                Button::new("manual-request")
                    .label("Request access")
                    .disabled(self.busy)
                    .on_click(cx.listener(|view, _, _, cx| {
                        let invitation = view.invitation.read(cx).value().to_string();
                        view.run(
                            "manual".into(),
                            move |service| {
                                let result = service.pair(invitation.trim())?;
                                Ok(result["status"].as_str().unwrap_or("Request sent").into())
                            },
                            cx,
                        );
                    })),
            )
            .child(self.feedback("manual"))
            .into_any_element()
    }
}

impl Render for DevicesWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let row_list = DeviceRow::list(&self.snapshot);
        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_3()
            .p_5()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(div().text_xl().child("Devices"))
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .items_center()
                            .when(self.scanning, |row| {
                                row.child(div().text_sm().child("Scanning…"))
                            })
                            .child(
                                Button::new("refresh")
                                    .icon(IconName::RefreshCw)
                                    .tooltip("Refresh devices")
                                    .loading(self.scanning)
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.run(
                                            "refresh".into(),
                                            |service| {
                                                service.discover()?;
                                                Ok(String::new())
                                            },
                                            cx,
                                        );
                                    })),
                            )
                            .child(
                                Button::new("add-device")
                                    .label(if self.show_add { "Close" } else { "Add device" })
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.show_add = !view.show_add;
                                        view.show_manual = false;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(self.feedback("refresh"))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(div().text_sm().child("Device name"))
                    .child(div().flex_1().child(Input::new(&self.name)))
                    .child(
                        Button::new("save-name")
                            .label("Save")
                            .disabled(self.busy)
                            .on_click(cx.listener(|view, _, _, cx| {
                                let name = view.name.read(cx).value().to_string();
                                view.run(
                                    "name".into(),
                                    move |service| {
                                        service.rename(name)?;
                                        Ok("Saved".into())
                                    },
                                    cx,
                                );
                            })),
                    )
                    .child(
                        Button::new("system-name")
                            .label("Use system name")
                            .disabled(self.busy || self.snapshot["name_override"].is_null())
                            .on_click(cx.listener(|view, _, window, cx| {
                                match view.service.use_system_name() {
                                    Ok(value) => {
                                        let name = value["name"].as_str().unwrap_or("").to_string();
                                        view.name.update(cx, |input, cx| {
                                            input.set_value(name, window, cx)
                                        });
                                        view.snapshot = view.service.snapshot();
                                        view.message_by_action.remove("name");
                                    }
                                    Err(error) => {
                                        view.message_by_action
                                            .insert("name".into(), error.to_string());
                                    }
                                }
                                cx.notify();
                            })),
                    ),
            )
            .child(self.feedback("name"))
            .when(self.snapshot["address"].is_null(), |column| {
                column.child(
                    div()
                        .text_sm()
                        .child("Device connections unavailable. Open Connection details."),
                )
            })
            .when(self.show_add, |column| {
                column.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .child(div().flex_1().child(Input::new(&self.host)))
                                .child(
                                    Button::new("find-device")
                                        .label("Find device")
                                        .disabled(self.busy)
                                        .on_click(cx.listener(|view, _, _, cx| {
                                            let host = view.host.read(cx).value().to_string();
                                            view.run(
                                                "find".into(),
                                                move |service| {
                                                    let description = service.find_device(&host)?;
                                                    Ok(format!(
                                                        "Verified {}.",
                                                        description["device"]["name"]
                                                            .as_str()
                                                            .unwrap_or("device")
                                                    ))
                                                },
                                                cx,
                                            );
                                        })),
                                ),
                        )
                        .child(self.feedback("find")),
                )
            })
            .child(
                div()
                    .id("device-content")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .when(!self.show_manual, |column| {
                        column.child(
                            div()
                                .border_1()
                                .border_color(cx.theme().border)
                                .rounded_md()
                                .overflow_hidden()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_3()
                                        .px_3()
                                        .py_2()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(div().flex_1().child("Device"))
                                        .child(div().w(px(180.)).child("Outgoing"))
                                        .child(div().w(px(220.)).child("Incoming")),
                                )
                                .when(row_list.is_empty(), |list| {
                                    list.child(
                                        div()
                                            .p_3()
                                            .child("No devices yet. Add a device or refresh."),
                                    )
                                })
                                .children(row_list.iter().map(|row| self.device_row(row, cx))),
                        )
                    })
                    .when(self.show_manual, |column| {
                        column.child(self.manual_panel(cx))
                    }),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        Button::new("manual")
                            .label(if self.show_manual {
                                "Back to devices"
                            } else {
                                "Pair manually"
                            })
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.show_manual = !view.show_manual;
                                view.show_add = false;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("details")
                            .label(if self.show_details {
                                "Hide details"
                            } else {
                                "Connection details"
                            })
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.show_details = !view.show_details;
                                cx.notify();
                            })),
                    ),
            )
            .when(self.show_details, |column| {
                column.child(
                    div()
                        .text_xs()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            self.snapshot["connection_status"]
                                .as_str()
                                .unwrap_or("Checking connection")
                                .to_string(),
                        )
                        .child(format!(
                            "Address {}",
                            self.snapshot["address"].as_str().unwrap_or("Unavailable")
                        ))
                        .child(format!(
                            "Identity {}",
                            self.snapshot["id"].as_str().unwrap_or("")
                        )),
                )
            })
    }
}

#[derive(Clone, Copy, Debug)]
enum DeviceAction {
    Request,
    Remove,
    Allow,
    Decline,
    Revoke,
    Retry,
}

impl DeviceAction {
    fn run(self, service: &DeviceService, id: &str) -> io::Result<String> {
        match self {
            Self::Request => {
                let result = service.request_access(id)?;
                Ok(result["status"].as_str().unwrap_or("Request sent").into())
            }
            Self::Remove => {
                service.remove_target(id)?;
                Ok("Outgoing connection removed".into())
            }
            Self::Allow => {
                service.approve(id)?;
                Ok("Incoming access allowed".into())
            }
            Self::Decline => {
                service.decline(id)?;
                Ok("Incoming request declined".into())
            }
            Self::Revoke => {
                service.revoke_access(id)?;
                Ok("Incoming access revoked".into())
            }
            Self::Retry => {
                service.allow_requests(id)?;
                Ok("New incoming requests allowed".into())
            }
        }
    }
}

#[derive(Default)]
struct OpenWindow(Option<AnyWindowHandle>);
impl Global for OpenWindow {}
