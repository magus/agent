use super::device_service::DeviceService;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::{prelude::*, *};
use serde_json::Value;
use std::io;
use std::sync::Arc;
use std::time::Duration;

pub(super) struct DevicesWindow {
    service: Arc<DeviceService>,
    name: Entity<InputState>,
    invitation: Entity<InputState>,
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
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(640.), px(720.)),
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
        self.message_by_action
            .insert(action_id.clone(), "Working…".into());
        let service = self.service.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { action(service) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.busy = false;
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

    fn device_row(&self, device: &Value, kind: &str, cx: &mut Context<Self>) -> AnyElement {
        let certificate: Vec<u8> =
            serde_json::from_value(device["certificate"].clone()).unwrap_or_default();
        let id = super::identity::device_id(&certificate);
        let name = device["name"].as_str().unwrap_or("Device").to_string();
        let key = format!("{kind}-{id}");
        let status = if kind == "target" {
            self.snapshot["status_by_id"][&id]
                .as_str()
                .unwrap_or("Checking connection")
        } else if kind == "pending" {
            "Wants to control this Mac"
        } else if kind == "allowed" {
            "Can control this Mac"
        } else if kind == "declined" {
            "Access denied"
        } else {
            "Shell and computer use"
        };
        let label = match kind {
            "target" => "Remove target",
            "allowed" => "Revoke access",
            "pending" => "Allow",
            "declined" => "Allow requests",
            _ => "Request access",
        };
        let operation = kind.to_string();
        let action_id = key.clone();
        let device_id = id.clone();
        div()
            .flex()
            .flex_col()
            .gap_1()
            .p_3()
            .rounded_md()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .child(
                        div().flex().flex_col().gap_1().child(name).child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(status.to_string()),
                        ),
                    )
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .child(
                                Button::new(SharedString::from(key.clone()))
                                    .label(label)
                                    .disabled(self.busy)
                                    .on_click(cx.listener(move |view, _, _, cx| {
                                        let id = device_id.clone();
                                        let operation = operation.clone();
                                        view.run(
                                            action_id.clone(),
                                            move |service| match operation.as_str() {
                                                "target" => {
                                                    service.remove_target(&id)?;
                                                    Ok("Target removed".into())
                                                }
                                                "allowed" => {
                                                    service.revoke_access(&id)?;
                                                    Ok("Access revoked".into())
                                                }
                                                "pending" => {
                                                    service.approve(&id)?;
                                                    Ok("Access allowed".into())
                                                }
                                                "declined" => {
                                                    service.allow_requests(&id)?;
                                                    Ok("New requests allowed".into())
                                                }
                                                _ => {
                                                    let result = service.request_access(&id)?;
                                                    Ok(result["status"]
                                                        .as_str()
                                                        .unwrap_or("Request sent")
                                                        .into())
                                                }
                                            },
                                            cx,
                                        );
                                    })),
                            )
                            .when(kind == "pending", |row| {
                                let action_id = key.clone();
                                row.child(
                                    Button::new(SharedString::from(format!("decline-{id}")))
                                        .label("Decline")
                                        .disabled(self.busy)
                                        .on_click(cx.listener(move |view, _, _, cx| {
                                            let id = id.clone();
                                            view.run(
                                                action_id.clone(),
                                                move |service| {
                                                    service.decline(&id)?;
                                                    Ok("Request declined".into())
                                                },
                                                cx,
                                            );
                                        })),
                                )
                            }),
                    ),
            )
            .child(self.feedback(&key))
            .into_any_element()
    }
}

impl Render for DevicesWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending_list = self.snapshot["pending_list"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let peer_list = self.snapshot["peer_list"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let candidate_list = self.snapshot["candidate_list"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let target_list = peer_list
            .iter()
            .filter(|peer| peer["target"] == true)
            .collect::<Vec<_>>();
        let allowed_list = peer_list
            .iter()
            .filter(|peer| peer["may_control_us"] == true)
            .collect::<Vec<_>>();
        let available_list = candidate_list
            .iter()
            .filter(|candidate| {
                !target_list.iter().any(|peer| {
                    peer["device"]["certificate"]
                        == candidate["description"]["device"]["certificate"]
                })
            })
            .collect::<Vec<_>>();

        div().size_full().flex().flex_col().bg(cx.theme().background).text_color(cx.theme().foreground)
            .child(div().p_6().pb_3().flex().flex_col().gap_2()
                .child(div().text_xl().child("Devices"))
                .child(div().text_sm().child("Device name"))
                .child(div().flex().gap_2().items_center()
                    .child(div().flex_1().child(Input::new(&self.name)))
                    .child(Button::new("save-name").label("Save").disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let name = view.name.read(cx).value().to_string();
                            view.run("name".into(), move |service| {
                                service.rename(name)?;
                                Ok("Saved".into())
                            }, cx);
                        }))))
                .child(self.feedback("name"))
                .child(div().flex().justify_between().items_center()
                    .child(div().text_sm().child(self.snapshot["connection_status"].as_str().unwrap_or("Checking Tailscale").to_string()))
                    .child(Button::new("refresh-devices").label("Refresh").disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            view.run("refresh".into(), |service| {
                                service.discover()?;
                                Ok("Devices refreshed".into())
                            }, cx);
                        }))))
                .child(self.feedback("refresh")))
            .child(div().id("device-list").flex_1().min_h_0().overflow_y_scroll().px_6().pb_6().flex().flex_col().gap_3()
                .when(!self.show_manual, |column| column
                .when(!pending_list.is_empty(), |column| column
                    .child(div().text_lg().child("Access requests"))
                    .children(pending_list.iter().map(|pending| self.device_row(&pending["device"], "pending", cx))))
                .child(div().text_lg().child("Targets"))
                .child(div().text_sm().text_color(cx.theme().muted_foreground).child("Devices this Mac can control"))
                .when(target_list.is_empty(), |column| column.child("No targets yet"))
                .children(target_list.iter().map(|peer| self.device_row(&peer["device"], "target", cx)))
                .child(div().text_lg().mt_3().child("Devices allowed to control this Mac"))
                .when(allowed_list.is_empty(), |column| column.child(div().text_sm().child("No devices have access")))
                .children(allowed_list.iter().map(|peer| self.device_row(&peer["device"], "allowed", cx)))
                .child(div().text_lg().mt_3().child("Available devices"))
                .when(available_list.is_empty(), |column| column.child(div().text_sm().child("No other Agent devices found. Keep Agent open on both devices.")))
                .children(available_list.iter().map(|candidate| self.device_row(&candidate["description"]["device"], "available", cx))))
                .child(Button::new("pair-manually").label(if self.show_manual { "Hide manual pairing" } else { "Pair manually" })
                    .on_click(cx.listener(|view, _, _, cx| { view.show_manual = !view.show_manual; cx.notify(); })))
                .when(self.show_manual, |column| column.child(div().flex().flex_col().gap_2().p_3().border_1().border_color(cx.theme().border).rounded_md()
                    .child("Let another device request access to this Mac")
                    .child(Button::new("copy-invitation").label("Copy invitation").disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let message = match view.service.invite() {
                                Ok(value) => {
                                    cx.write_to_clipboard(ClipboardItem::new_string(value["invitation"].as_str().unwrap().into()));
                                    "Copied. Expires in 10 minutes. Approval is still required.".into()
                                }
                                Err(error) => error.to_string(),
                            };
                            view.message_by_action.insert("copy".into(), message);
                            cx.notify();
                        })))
                    .child(self.feedback("copy"))
                    .child("Or paste an invitation to control another device")
                    .child(Input::new(&self.invitation))
                    .child(Button::new("manual-request").label("Request access").disabled(self.busy)
                        .on_click(cx.listener(|view, _, _, cx| {
                            let invitation = view.invitation.read(cx).value().to_string();
                            view.run("manual".into(), move |service| {
                                let result = service.pair(invitation.trim())?;
                                Ok(result["status"].as_str().unwrap_or("Request sent").into())
                            }, cx);
                        })))
                    .child(self.feedback("manual"))))
                .child(Button::new("connection-details").label(if self.show_details { "Hide connection details" } else { "Connection details" })
                    .on_click(cx.listener(|view, _, _, cx| { view.show_details = !view.show_details; cx.notify(); })))
                .when(self.show_details, |column| {
                    column.child(div().text_xs().child(format!("Address {}", self.snapshot["address"].as_str().unwrap_or("Unavailable"))))
                        .child(div().text_xs().child(format!("Identity {}", self.snapshot["id"].as_str().unwrap_or(""))))
                        .children(peer_list.iter().filter(|peer| {
                            let certificate: Vec<u8> = serde_json::from_value(peer["device"]["certificate"].clone()).unwrap_or_default();
                            let id = super::identity::device_id(&certificate);
                            self.snapshot["declined_id_list"].as_array().is_some_and(|list| list.iter().any(|value| value.as_str() == Some(&id)))
                        }).map(|peer| self.device_row(&peer["device"], "declined", cx)))
                }))
    }
}

#[derive(Default)]
struct OpenWindow(Option<AnyWindowHandle>);
impl Global for OpenWindow {}
