use super::chat_client::ChatClient;
use super::composer::Composer;
use super::conversation_notice::conversation_notice;
use super::message_bubble::message_bubble;
use super::pending_message::PendingMessage;
use super::tool_row::ToolRow;
use crate::protocol::{ConversationView, Entry, ModelOption, ModelSettings};
use gpui_kit::base::TextSelection;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{InputEvent, TextareaState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{ActiveTheme, Disableable, Selectable, Sizable};
use gpui_kit::{prelude::*, *};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

pub(super) struct ChatWindow {
    agent: Arc<ChatClient>,
    _typing_subscription: Subscription,
    prompt: Entity<TextareaState>,
    model_option_list: Vec<ModelOption>,
    settings: ModelSettings,
    saving_settings: bool,
    loading: bool,
    sending: bool,
    pending_message: Option<PendingMessage>,
    saving_devices: bool,
    revision: u64,
    was_running: bool,
    scroll: ScrollHandle,
    expanded_entry_set: std::collections::HashSet<String>,
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
                            if !view.saving_settings {
                                view.settings = ModelSettings {
                                    model: conversation.model.clone(),
                                    reasoning_effort: conversation.reasoning_effort.clone(),
                                };
                                if view.settings.model.is_empty()
                                    && let Some(option) = view.model_option_list.first()
                                {
                                    view.settings.model = option.model.clone();
                                }
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
            model_option_list: Vec::new(),
            settings: ModelSettings {
                model: conversation.model.clone(),
                reasoning_effort: conversation.reasoning_effort.clone(),
            },
            saving_settings: false,
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
                .spawn(async move { client.model_option_list() })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.loading = false;
                match result {
                    Ok(option_list) => {
                        if view.settings.model.is_empty()
                            && let Some(option) = option_list.first()
                        {
                            view.settings.model = option.model.clone();
                        }
                        view.model_option_list = option_list;
                    }
                    Err(error) => view.message = error.to_string(),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn save_settings(&mut self, settings: ModelSettings, cx: &mut Context<Self>) {
        if self.saving_settings || self.conversation.id.is_empty() {
            return;
        }
        self.settings = settings.clone();
        self.saving_settings = true;
        let service = self.service_host.clone();
        let id = self.conversation.id.clone();
        let agent = self.agent.clone();
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { agent.set_conversation_model(&service, &id, settings) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.saving_settings = false;
                if let Err(error) = result {
                    view.message = error.to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
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
            || self.settings.model.is_empty()
            || self.saving_settings
            || self.conversation.pending()
        {
            return;
        }
        self.pending_message = Some(PendingMessage {
            service_host: self.service_host.clone(),
            conversation_id: self.conversation.id.clone(),
            message_id: uuid::Uuid::new_v4().to_string(),
            prompt,
            settings: self.settings.clone(),
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
                        pending.settings,
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
            .gap_2()
            .px_5()
            .py_4();
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
            history = history.child(
                div()
                    .flex_1()
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(cx.theme().muted_foreground)
                    .child(if self.conversation.id.is_empty() {
                        "Connect to a service, then create a conversation."
                    } else {
                        "What would you like to do?"
                    }),
            );
        }
        for (index, entry) in entry_list.iter().enumerate() {
            let mut item = div()
                .id(("entry", index))
                .w_full()
                .max_w(px(900.))
                .mx_auto()
                .min_w_0()
                .flex_shrink_0()
                .flex()
                .flex_col()
                .gap_2();
            if let Some(request) = &entry.request {
                let key = format!(
                    "{}:{}:{}",
                    self.service_host, self.conversation.id, request.id
                );
                let expanded = self.expanded_entry_set.contains(&key);
                let view = cx.entity().downgrade();
                let device_name = available_target_list
                    .iter()
                    .find(|device| device.id == request.host)
                    .map(|device| device.name.clone())
                    .unwrap_or_else(|| request.host.chars().take(8).collect());
                item = item.child(ToolRow {
                    entry: entry.clone(),
                    device_name,
                    expanded,
                    on_toggle: Rc::new(move |cx| {
                        let _ = view.update(cx, |view, cx| {
                            if !view.expanded_entry_set.remove(&key) {
                                view.expanded_entry_set.insert(key.clone());
                            }
                            cx.notify();
                        });
                    }),
                });
            } else {
                item = item.my_2().child(message_bubble(
                    index,
                    entry.text.clone(),
                    entry.role == "You",
                ));
            }
            let delivery_failed = !self.sending
                && pending.is_some_and(|pending| {
                    entry.message_id.as_deref() == Some(&pending.message_id)
                });
            if delivery_failed {
                item = item.child(
                    div()
                        .flex()
                        .justify_end()
                        .items_center()
                        .gap_2()
                        .text_sm()
                        .text_color(rgb(0xe59a90))
                        .child("Delivery not confirmed")
                        .child(
                            Button::new("retry-message")
                                .ghost()
                                .compact()
                                .label("Retry")
                                .on_click(cx.listener(|view, _, _, cx| view.send_pending(cx))),
                        ),
                );
            }
            history = history.child(item);
        }
        let waiting_for_model = (self.sending || running) && !self.conversation.pending();
        if waiting_for_model {
            history = history.child(
                div()
                    .id("model-activity")
                    .w_full()
                    .max_w(px(900.))
                    .mx_auto()
                    .flex_shrink_0()
                    .px_4()
                    .h(px(40.))
                    .flex()
                    .items_center()
                    .gap_2()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(Spinner::new().small())
                    .child("Thinking"),
            );
        }
        let empty = self.prompt.read(cx).value().trim().is_empty();
        let stopping = running && empty;
        let cannot_send = empty
            || self.sending
            || self.pending_message.is_some()
            || running
            || self.loading
            || self.saving_settings
            || self.settings.model.is_empty()
            || self.conversation.id.is_empty()
            || self.conversation.pending();
        let settings_view = cx.entity().downgrade();
        let send_view = cx.entity().downgrade();
        let refresh_view = cx.entity().downgrade();
        let status = if self.message.is_empty() {
            conversation_notice(&self.conversation.status)
        } else {
            conversation_notice(&self.message)
        };
        let can_retrieve = self.conversation.pending() && !running;
        let needs_notice = !status.is_empty() || can_retrieve;
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(rgb(0x191a1c))
            .text_color(rgb(0xe8e9ed))
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
                    .px_4()
                    .py_2()
                    .child(
                        Button::new("service")
                            .ghost()
                            .compact()
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
                            .ghost()
                            .compact()
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
                            .ghost()
                            .compact()
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
                    .flex_col()
                    .gap_2()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Devices"),
                    )
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
                                .compact()
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
                div().flex_shrink_0().px_3().pb_3().child(
                    div()
                        .w_full()
                        .max_w(px(900.))
                        .mx_auto()
                        .relative()
                        .when(needs_notice, |view| {
                            view.child(
                                div()
                                    .absolute()
                                    .bottom_full()
                                    .left_0()
                                    .right_0()
                                    .mb_2()
                                    .p_2()
                                    .rounded_md()
                                    .bg(rgb(0x28292c))
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(div().flex_1().min_w_0().child(status.to_owned()))
                                    .when(can_retrieve, |view| {
                                        view.child(
                                            Button::new("retrieve")
                                                .ghost()
                                                .compact()
                                                .label("Retrieve results")
                                                .on_click(cx.listener(|view, _, _, cx| {
                                                    view.perform(cx, |agent| agent.retrieve())
                                                })),
                                        )
                                    }),
                            )
                        })
                        .child(Composer {
                            prompt: self.prompt.clone(),
                            option_list: self.model_option_list.clone(),
                            settings: self.settings.clone(),
                            loading: self.loading,
                            saving: self.saving_settings,
                            can_configure: !self.conversation.id.is_empty(),
                            cannot_send,
                            stopping,
                            on_settings: Rc::new(move |settings, cx| {
                                let _ = settings_view
                                    .update(cx, |view, cx| view.save_settings(settings, cx));
                            }),
                            on_refresh: Rc::new(move |cx| {
                                let _ = refresh_view.update(cx, |view, cx| view.load_models(cx));
                            }),
                            on_submit: Rc::new(move |window, cx| {
                                let _ = send_view.update(cx, |view, cx| {
                                    if stopping {
                                        view.message = "Stopping…".into();
                                        view.perform(cx, |agent| agent.stop());
                                    } else {
                                        view.send(window, cx);
                                    }
                                });
                            }),
                        }),
                ),
            )
    }
}

#[derive(Default)]
struct OpenWindow(Option<AnyWindowHandle>);
impl Global for OpenWindow {}
