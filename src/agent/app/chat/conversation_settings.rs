use super::chat_client::ChatClient;
use crate::protocol::{ModelOption, ModelSettings};
use gpui_kit::component::button::Button;
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{ActiveTheme, Disableable};
use gpui_kit::{prelude::*, *};
use std::sync::Arc;
use std::time::Duration;

pub(super) struct ConversationSettings {
    agent: Arc<ChatClient>,
    saving: bool,
    model_option_list: Vec<ModelOption>,
    loading: bool,
    message: String,
}

impl ConversationSettings {
    pub fn open(agent: Arc<ChatClient>, cx: &mut App) {
        if crate::app::core::window::activate(cx.default_global::<OpenWindow>().0, cx) {
            return;
        }

        let options = WindowOptions {
            window_min_size: Some(size(px(480.), px(320.))),
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                None,
                size(px(600.), px(600.)),
                cx,
            ))),
            titlebar: Some(TitlebarOptions {
                title: Some("Agent · Conversation settings".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        match gpui_kit::open_window(options, cx, |_, cx| {
            cx.new(|cx: &mut Context<Self>| {
                cx.spawn(async move |view, cx| {
                    loop {
                        cx.background_executor()
                            .timer(Duration::from_millis(500))
                            .await;
                        if view.update(cx, |_, cx| cx.notify()).is_err() {
                            break;
                        }
                    }
                })
                .detach();

                let client = agent.clone();
                cx.spawn(async move |view, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move { client.model_option_list() })
                        .await;
                    let _ = view.update(cx, |view, cx| {
                        view.loading = false;
                        match result {
                            Ok(option_list) => view.model_option_list = option_list,
                            Err(error) => view.message = error.to_string(),
                        }
                        cx.notify();
                    });
                })
                .detach();

                Self {
                    model_option_list: Vec::new(),
                    loading: true,
                    agent,
                    saving: false,
                    message: String::new(),
                }
            })
        }) {
            Ok((handle, view)) => {
                cx.global_mut::<OpenWindow>().0 = Some(handle);
                crate::app::core::window::observe(handle, &view, cx);
            }
            Err(error) => eprintln!("Could not open conversation settings. {error}"),
        }
        cx.activate(true);
    }

    fn save_model(&mut self, settings: ModelSettings, cx: &mut Context<Self>) {
        self.saving = true;
        self.message.clear();
        let agent = self.agent.clone();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { agent.set_model_settings(settings) })
                .await;
            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                if let Err(error) = result {
                    view.message = error.to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn save(&mut self, device_list: Vec<String>, cx: &mut Context<Self>) {
        self.saving = true;
        self.message.clear();
        let agent = self.agent.clone();

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { agent.set_targets(device_list, true) })
                .await;

            let _ = view.update(cx, |view, cx| {
                view.saving = false;
                if let Err(error) = result {
                    view.message = error.to_string();
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }
}

impl Render for ConversationSettings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = self.agent.model_settings();
        let model = if settings.model.is_empty() {
            self.model_option_list
                .first()
                .map(|option| option.model.clone())
                .unwrap_or_default()
        } else {
            settings.model.clone()
        };
        let effort_list = self
            .model_option_list
            .iter()
            .find(|option| option.model == model)
            .map(|option| option.reasoning_effort_list.clone())
            .unwrap_or_default();
        let model_label = if self.loading {
            "Loading models…".into()
        } else if model.is_empty() {
            "No models available".into()
        } else {
            model.clone()
        };
        let reasoning_label = settings
            .reasoning_effort
            .clone()
            .unwrap_or_else(|| "Automatic".into());
        let option_list = self.model_option_list.clone();
        let model_view = cx.entity().downgrade();
        let reasoning_view = cx.entity().downgrade();
        let disabled = self.loading || self.saving || option_list.is_empty();
        let device_list = self.agent.available_target_list();
        let enabled_device_list = self.agent.default_target_list();
        let mut rows = div()
            .flex()
            .flex_col()
            .border_1()
            .border_color(cx.theme().border)
            .rounded_md();

        if device_list.is_empty() {
            rows = rows.child(div().p_4().child("No devices available"));
        }
        for device in device_list {
            let enabled = enabled_device_list.contains(&device.id);
            let mut updated_device_list = enabled_device_list.clone();
            if enabled {
                updated_device_list.retain(|id| id != &device.id);
            } else {
                updated_device_list.push(device.id.clone());
            }

            rows = rows.child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap_3()
                    .p_3()
                    .child(device.name)
                    .child(
                        Button::new(SharedString::from(device.id))
                            .label(if enabled { "Enabled" } else { "Disabled" })
                            .disabled(self.saving)
                            .on_click(cx.listener(move |view, _, _, cx| {
                                view.save(updated_device_list.clone(), cx);
                            })),
                    ),
            );
        }

        div().id("conversation-settings").size_full().overflow_y_scroll()
            .flex().flex_col().gap_4().p_6()
            .bg(cx.theme().background).text_color(cx.theme().foreground)
            .child(div().text_xl().child("Conversation settings"))
            .child(format!("Applies to conversations started from {} on {}. Existing conversations are unchanged.",
                self.agent.sender_name(), self.agent.service_name()))
            .child(div().flex().items_center().justify_between().gap_3()
                .child("Default model")
                .child(Button::new("default-model").label(model_label).disabled(disabled)
                    .dropdown_menu(move |mut menu, _, _| {
                        for option in &option_list {
                            let option = option.clone();
                            let view = model_view.clone();
                            menu = menu.item(PopupMenuItem::new(option.model.clone()).on_click(move |_, _, cx| {
                                let _ = view.update(cx, |view, cx| {
                                    let mut settings = view.agent.model_settings();
                                    settings.model = option.model.clone();
                                    if settings.reasoning_effort.as_ref().is_some_and(|effort| !option.reasoning_effort_list.contains(effort)) {
                                        settings.reasoning_effort = None;
                                    }
                                    view.save_model(settings, cx);
                                });
                            }));
                        }
                        menu
                    })))
            .child(div().flex().items_center().justify_between().gap_3()
                .child("Default reasoning level")
                .child(Button::new("default-reasoning").label(reasoning_label).disabled(disabled)
                    .dropdown_menu(move |mut menu, _, _| {
                        let mut choice_list = vec![("Automatic".to_string(), None)];
                        choice_list.extend(effort_list.iter().map(|effort| (effort.clone(), Some(effort.clone()))));
                        for (label, effort) in choice_list {
                            let view = reasoning_view.clone();
                            let model = model.clone();
                            menu = menu.item(PopupMenuItem::new(label).on_click(move |_, _, cx| {
                                let _ = view.update(cx, |view, cx| {
                                    view.save_model(ModelSettings { model: model.clone(), reasoning_effort: effort.clone() }, cx);
                                });
                            }));
                        }
                        menu
                    })))
            .child(div().font_weight(FontWeight::SEMIBOLD).child("Default devices"))
            .child(rows)
            .when(self.saving, |view| view.child("Saving…"))
            .when(!self.message.is_empty(), |view| view.child(self.message.clone()))
    }
}

#[derive(Default)]
struct OpenWindow(Option<AnyWindowHandle>);
impl Global for OpenWindow {}
