use super::chat_client::ChatClient;
use super::pending_message::PendingMessage;
use super::select_model::select_model;
use crate::protocol::{ConversationView, Entry};
use gpui_kit::base::{SelectableText, TextSelection};
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{ActiveTheme, Disableable, IconName, Selectable};
use gpui_kit::{prelude::*, *};
use std::sync::Arc;
use std::time::Duration;

pub(super) struct ChatWindow {
    agent: Arc<ChatClient>,
    _typing_subscription: Subscription,
    prompt: Entity<TextareaState>,
    model_list: Vec<String>,
    model: String,
    loading: bool,
    sending: bool,
    pending_message: Option<PendingMessage>,
    saving_devices: bool,
    revision: u64,
    was_running: bool,
    scroll: ScrollHandle,
    expanded_entry_set: std::collections::HashSet<usize>,
    message: String,
    conversation: ConversationView,
    service_host: String,
}

impl ChatWindow {
    pub fn open(agent: Arc<ChatClient>, cx: &mut App) {
        if crate::app::core::window::activate(cx.default_global::<OpenWindow>().0, cx) {
            return;
        }
        let options = WindowOptions {
            window_min_size: Some(size(px(720.), px(500.))),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(960.), px(720.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Agent".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        match gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| Self::new(agent, window, cx))
        }) {
            Ok((handle, view)) => {
                cx.global_mut::<OpenWindow>().0 = Some(handle);
                crate::app::core::window::observe(handle, &view, cx);
            }
            Err(error) => eprintln!("Could not open chat. {error}"),
        }
        cx.activate(true);
    }

    fn new(agent: Arc<ChatClient>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let conversation = agent.view();
        cx.spawn(async move |view, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(100))
                    .await;
                if view
                    .update(cx, |view, cx| {
                        let service_host = view.agent.service_host().unwrap_or_default();
                        if view.service_host != service_host {
                            view.service_host = service_host.clone();
                            view.load_models(cx);
                        }
                        let revision = view.agent.revision();
                        let running = view.agent.running();
                        if revision != view.revision || running != view.was_running {
                            let at_bottom =
                                view.scroll.offset().y + view.scroll.max_offset().y <= px(20.);
                            let conversation = view.agent.view();
                            if conversation.id != view.conversation.id {
                                view.model = select_model(
                                    &conversation.model,
                                    &view.model,
                                    &view.model_list,
                                );
                            }
                            view.conversation = conversation;
                            if at_bottom {
                                view.scroll.scroll_to_bottom();
                            }
                            view.revision = revision;
                            view.was_running = running;
                            if !running && view.message == "Stopping…" {
                                view.message.clear();
                            }
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
        let service_host = agent.service_host().unwrap_or_default();
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(1, 6)
                .submit_on_enter(true)
                .placeholder("Message Agent…")
        });
        cx.subscribe_in(&prompt, window, |view, _, event, window, cx| match event {
            InputEvent::PressEnter { shift: false, .. } => view.send(window, cx),
            InputEvent::Change => cx.notify(),
            _ => {}
        })
        .detach();

        let chat_window = window.window_handle().window_id();
        let view = cx.entity().downgrade();
        let typing_subscription = cx.intercept_keystrokes(move |event, window, cx| {
            if window.window_handle().window_id() == chat_window {
                let _ = view.update(cx, |view, cx| view.focus_for_typing(event, window, cx));
            }
        });

        let mut view = Self {
            _typing_subscription: typing_subscription,
            agent,
            prompt,
            model_list: Vec::new(),
            model: conversation.model.clone(),
            loading: false,
            sending: false,
            pending_message: None,
            saving_devices: false,
            revision: 0,
            was_running: false,
            scroll: ScrollHandle::new(),
            expanded_entry_set: std::collections::HashSet::new(),
            message: String::new(),
            conversation,
            service_host,
        };
        view.scroll.scroll_to_bottom();
        view.load_models(cx);
        view
    }

    fn focus_composer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.prompt.update(cx, |input, cx| input.focus(window, cx));
    }

    fn focus_for_typing(
        &mut self,
        event: &KeystrokeEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let modifiers = event.keystroke.modifiers;
        let focus_shortcut = event.keystroke.key == "escape"
            && modifiers.shift
            && !modifiers.control
            && !modifiers.platform
            && !modifiers.alt;
        if focus_shortcut {
            self.focus_composer(window, cx);
            window.prevent_default();
            cx.stop_propagation();
            return;
        }

        let input_or_menu_focused = event
            .context_stack
            .iter()
            .any(|context| context.contains("Input") || context.contains("PopupMenu"));
        let copy_shortcut = event.keystroke.key == "c"
            && modifiers.platform
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.shift;
        if copy_shortcut && !input_or_menu_focused {
            let selected_text = TextSelection::selected_text(window, cx);
            if !selected_text.is_empty() {
                cx.write_to_clipboard(ClipboardItem::new_string(selected_text));
                window.prevent_default();
                cx.stop_propagation();
            }
            return;
        }

        if input_or_menu_focused || modifiers.control || modifiers.platform {
            return;
        }

        let Some(text) = event.keystroke.key_char.as_ref().filter(|text| {
            !text.is_empty() && text.chars().all(|character| !character.is_control())
        }) else {
            return;
        };

        self.prompt.update(cx, |input, cx| {
            input.focus(window, cx);
            input.replace(text.clone(), window, cx);
        });
        window.prevent_default();
        cx.stop_propagation();
        cx.notify();
    }

    fn load_models(&mut self, cx: &mut Context<Self>) {
        self.loading = true;
        self.message.clear();
        let client = self.agent.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { client.model_list() })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.loading = false;
                match result {
                    Ok(model_list) => {
                        view.model = select_model("", &view.model, &model_list);
                        view.model_list = model_list;
                    }
                    Err(error) => view.message = error.to_string(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn set_devices(&mut self, device_list: Vec<String>, cx: &mut Context<Self>) {
        if self.saving_devices {
            return;
        }
        self.saving_devices = true;
        self.message.clear();
        let agent = self.agent.clone();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { agent.set_targets(device_list, false) })
                .await;

            let _ = view.update(cx, |view, cx| {
                view.saving_devices = false;
                if let Err(error) = result {
                    view.message = error.to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn perform(
        &mut self,
        cx: &mut Context<Self>,
        operation: impl FnOnce(Arc<ChatClient>) -> std::io::Result<()> + Send + 'static,
    ) {
        let agent = self.agent.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { operation(agent) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.message = result
                    .err()
                    .map(|error| error.to_string())
                    .unwrap_or_default();
                cx.notify();
            });
        })
        .detach();
    }

    fn send(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let prompt = self.prompt.read(cx).value().to_string();
        if prompt.trim().is_empty()
            || self.agent.running()
            || self.loading
            || self.sending
            || self.pending_message.is_some()
            || self.model.is_empty()
            || self.conversation.pending()
        {
            return;
        }
        self.pending_message = Some(PendingMessage {
            service_host: self.service_host.clone(),
            conversation_id: self.conversation.id.clone(),
            message_id: uuid::Uuid::new_v4().to_string(),
            prompt,
            model: self.model.clone(),
        });
        self.prompt
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.scroll.scroll_to_bottom();
        self.send_pending(cx);
    }

    fn send_pending(&mut self, cx: &mut Context<Self>) {
        if self.sending {
            return;
        }
        let Some(pending) = self.pending_message.clone() else {
            return;
        };
        let agent = self.agent.clone();
        self.sending = true;
        self.message.clear();
        cx.notify();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    agent.start(
                        pending.service_host,
                        pending.conversation_id,
                        pending.message_id,
                        pending.prompt,
                        pending.model,
                    )
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.sending = false;
                match result {
                    Ok(message_id) => {
                        if let Some(pending) = &mut view.pending_message {
                            pending.message_id = message_id;
                        }
                        view.conversation = view.agent.view();
                        if view.pending_message.as_ref().is_some_and(|pending| {
                            pending.confirmed(&view.service_host, &view.conversation)
                        }) {
                            view.pending_message = None;
                        }
                        view.message.clear();
                    }
                    Err(error) => view.message = format!("Could not confirm delivery. {error}"),
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for ChatWindow {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let running = self.agent.running();
        let target_list = self.agent.target_list();
        let available_target_list = self.agent.available_target_list();
        let model_list = self.model_list.clone();
        let model_view = cx.entity().downgrade();
        let model_label = if self.loading {
            "Loading models…".into()
        } else if self.model.is_empty() {
            "Choose model".into()
        } else {
            self.model.clone()
        };
        let mut history = div()
            .id("history")
            .capture_any_mouse_down(cx.listener(|_, event: &MouseDownEvent, window, cx| {
                if event.button == MouseButton::Left {
                    window.blur(cx);
                }
            }))
            .track_scroll(&self.scroll)
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap_5()
            .p_5();
        let mut entry_list = self.conversation.entry_list.clone();
        let pending = self.pending_message.as_ref().filter(|pending| {
            pending.service_host == self.service_host
                && pending.conversation_id == self.conversation.id
        });
        if let Some(pending) = pending
            && !pending.confirmed(&self.service_host, &self.conversation)
        {
            let mut entry = Entry::text("You", &pending.prompt);
            entry.message_id = Some(pending.message_id.clone());
            entry_list.push(entry);
        }
        if entry_list.is_empty() {
            history = history.child(div().text_color(cx.theme().muted_foreground).child(
                if self.conversation.id.is_empty() {
                    "Connect to a service, then create a conversation."
                } else {
                    "Send a message to begin."
                },
            ));
        }
        for (index, entry) in entry_list.iter().enumerate() {
            let mut item = div()
                .id(("entry", index))
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap_2();
            let mut heading = entry.role.clone();
            if let Some(request) = &entry.request {
                let name = available_target_list
                    .iter()
                    .find(|target| target.id == request.host)
                    .map(|target| target.name.as_str())
                    .unwrap_or(&request.host);
                heading = format!("Tool · {name} · {}", request.status);
            }
            item = item.child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .child(SelectableText::new(("entry-role", index), heading)),
            );
            let computer = entry
                .request
                .as_ref()
                .is_some_and(|request| request.tool["tool"] == "computer");
            let expanded = self.expanded_entry_set.contains(&index);
            let text = if computer && !expanded {
                serde_json::to_string(&entry.request.as_ref().unwrap().tool["action"])
                    .unwrap_or_default()
            } else {
                entry.text.clone()
            };
            if entry.role == "Agent" {
                item = item.child(TextView::markdown(("message", index), text).selectable(true));
            } else {
                item = item.child(
                    div()
                        .text_sm()
                        .child(SelectableText::new(("entry-text", index), text)),
                );
            }
            let delivery_failed = !self.sending
                && pending.is_some_and(|pending| {
                    entry.message_id.as_deref() == Some(&pending.message_id)
                });
            if delivery_failed {
                item = item.child(
                    div()
                        .flex()
                        .items_center()
                        .gap_3()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Delivery not confirmed")
                        .child(
                            Button::new("retry-message")
                                .label("Retry")
                                .on_click(cx.listener(|view, _, _, cx| view.send_pending(cx))),
                        ),
                );
            }
            if computer {
                item = item.child(
                    Button::new(("details", index))
                        .label(if expanded { "Hide details" } else { "Details" })
                        .on_click(cx.listener(move |view, _, _, cx| {
                            if !view.expanded_entry_set.remove(&index) {
                                view.expanded_entry_set.insert(index);
                            }
                            cx.notify();
                        })),
                );
            }
            if let Some(path) = &entry.image {
                item = item.child(
                    img(path.clone())
                        .w_full()
                        .h(px(360.))
                        .object_fit(ObjectFit::Contain),
                );
            }
            history = history.child(item);
        }
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
            .child({
                let service_list = self.agent.service_list();
                let conversation_list = self.agent.conversation_list();
                let service_view = cx.entity().downgrade();
                let conversation_view = cx.entity().downgrade();
                let selected_title = conversation_list
                    .iter()
                    .find(|record| record.id == self.conversation.id)
                    .map(|record| record.title.as_str())
                    .unwrap_or("Conversations")
                    .to_string();
                div()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_3()
                    .p_4()
                    .child(
                        Button::new("service")
                            .label(self.agent.service_name())
                            .disabled(self.sending)
                            .dropdown_menu(move |mut menu, _, _| {
                                for target in &service_list {
                                    let id = target.id.clone();
                                    let view = service_view.clone();
                                    menu = menu.item(
                                        PopupMenuItem::new(target.name.clone()).on_click(
                                            move |_, _, cx| {
                                                let id = id.clone();
                                                let _ = view.update(cx, |view, cx| {
                                                    view.perform(cx, move |agent| agent.connect(id))
                                                });
                                            },
                                        ),
                                    );
                                }
                                menu
                            }),
                    )
                    .child(
                        Button::new("conversations")
                            .label(selected_title)
                            .disabled(self.sending)
                            .dropdown_menu(move |mut menu, _, _| {
                                for record in &conversation_list {
                                    let id = record.id.clone();
                                    let title = record.title.clone();
                                    let view = conversation_view.clone();
                                    menu = menu.item(PopupMenuItem::new(title).on_click(
                                        move |_, _, cx| {
                                            let id = id.clone();
                                            let _ = view.update(cx, |view, cx| {
                                                view.perform(cx, move |agent| agent.select(id))
                                            });
                                        },
                                    ));
                                }
                                menu
                            }),
                    )
                    .child(
                        Button::new("new-conversation")
                            .label("New conversation")
                            .disabled(self.sending)
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.perform(cx, |agent| agent.create())
                            })),
                    )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .flex_wrap()
                    .gap_3()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        Button::new("model")
                            .label(model_label)
                            .disabled(running || self.loading)
                            .dropdown_menu(move |mut menu, _, _| {
                                for model in &model_list {
                                    let model = model.clone();
                                    let view = model_view.clone();
                                    menu = menu.item(PopupMenuItem::new(model.clone()).on_click(
                                        move |_, _, cx| {
                                            let _ = view.update(cx, |view, cx| {
                                                view.model = model.clone();
                                                cx.notify();
                                            });
                                        },
                                    ));
                                }
                                menu
                            }),
                    )
                    .child(
                        Button::new("refresh-models")
                            .label("Refresh models")
                            .disabled(self.loading || running)
                            .on_click(cx.listener(|view, _, _, cx| view.load_models(cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(div().font_weight(FontWeight::SEMIBOLD).child("Devices"))
                    .child(div().flex().flex_wrap().gap_2().children(
                        available_target_list.iter().map(|device| {
                            let enabled = target_list.iter().any(|allowed| allowed.id == device.id);
                            let mut device_list: Vec<String> =
                                target_list.iter().map(|device| device.id.clone()).collect();
                            if enabled {
                                device_list.retain(|id| id != &device.id);
                            } else {
                                device_list.push(device.id.clone());
                            }
                            let tooltip = if enabled {
                                format!(
                                    "{} is enabled. Click to disable for this conversation.",
                                    device.name
                                )
                            } else {
                                format!(
                                    "{} is disabled. Click to enable for this conversation.",
                                    device.name
                                )
                            };

                            Button::new(SharedString::from(format!("device-{}", device.id)))
                                .label(device.name.clone())
                                .selected(enabled)
                                .text_color(if enabled {
                                    cx.theme().foreground
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .tooltip(tooltip)
                                .disabled(self.saving_devices || self.conversation.id.is_empty())
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    view.set_devices(device_list.clone(), cx);
                                }))
                        }),
                    ))
                    .when(target_list.is_empty(), |view| {
                        view.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child("No devices enabled · Chat only"),
                        )
                    }),
            )
            .child(history)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .p_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .when(!self.message.is_empty(), |view| {
                        view.child(self.message.clone())
                    })
                    .when(!self.conversation.status.is_empty(), |view| {
                        view.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(self.conversation.status.clone()),
                        )
                    })
                    .when(self.conversation.pending(), |view| {
                        view.child(
                            Button::new("retrieve")
                                .label("Retrieve results")
                                .disabled(running)
                                .on_click(cx.listener(|view, _, _, cx| {
                                    view.perform(cx, |agent| agent.retrieve());
                                })),
                        )
                    })
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_3()
                            .p_2()
                            .rounded_lg()
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div().flex_1().child(
                                    Textarea::new(&self.prompt)
                                        .appearance(false)
                                        .bordered(false),
                                ),
                            )
                            .child({
                                let empty = self.prompt.read(cx).value().trim().is_empty();
                                let stopping = running && empty;
                                let cannot_send = empty
                                    || self.sending
                                    || self.pending_message.is_some()
                                    || running
                                    || self.loading
                                    || self.model.is_empty()
                                    || self.conversation.pending();

                                Button::new("send-stop")
                                    .icon(if stopping {
                                        IconName::Square
                                    } else {
                                        IconName::ArrowUp
                                    })
                                    .accessibility_label(if stopping { "Stop" } else { "Send" })
                                    .tooltip(if stopping { "Stop" } else { "Send · Enter" })
                                    .rounded_full()
                                    .disabled(!stopping && cannot_send)
                                    .on_click(cx.listener(move |view, _, window, cx| {
                                        if stopping {
                                            view.message = "Stopping…".into();
                                            view.perform(cx, |agent| agent.stop());
                                        } else {
                                            view.send(window, cx);
                                        }
                                    }))
                            }),
                    ),
            )
    }
}

#[derive(Default)]
struct OpenWindow(Option<AnyWindowHandle>);
impl Global for OpenWindow {}
