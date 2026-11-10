use crate::protocol::{ModelOption, ModelSettings};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Textarea, TextareaState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Disableable, IconName};
use gpui_kit::{prelude::*, *};
use std::rc::Rc;

#[derive(IntoElement)]
pub(super) struct Composer {
    pub prompt: Entity<TextareaState>,
    pub option_list: Vec<ModelOption>,
    pub settings: ModelSettings,
    pub loading: bool,
    pub saving: bool,
    pub can_configure: bool,
    pub cannot_send: bool,
    pub stopping: bool,
    pub on_settings: Rc<SettingsHandler>,
    pub on_refresh: Rc<dyn Fn(&mut App)>,
    pub on_submit: Rc<SubmitHandler>,
}

impl RenderOnce for Composer {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let effort_list = self
            .option_list
            .iter()
            .find(|option| option.model == self.settings.model)
            .map(|option| option.reasoning_effort_list.clone())
            .unwrap_or_default();
        let disabled = self.loading || self.saving || !self.can_configure;
        let model_text = if self.settings.model.is_empty() {
            if self.loading {
                "Loading models".into()
            } else {
                "Choose model".into()
            }
        } else {
            model_label(&self.settings.model)
        };
        let effort_text = effort_label(self.settings.reasoning_effort.as_deref());
        let on_model = self.on_settings.clone();
        let on_effort = self.on_settings;
        let settings = self.settings.clone();
        let model = self.settings.model;

        let effort_icon = effort_signal(self.settings.reasoning_effort.as_deref());
        let controls = div()
            .flex()
            .items_center()
            .justify_end()
            .gap_1()
            .h(px(28.))
            .child(
                Button::new("model")
                    .ghost()
                    .compact()
                    .label(model_text)
                    .accessibility_label("Model")
                    .tooltip("Model for the next turn")
                    .disabled(disabled)
                    .dropdown_menu(move |mut menu, _, _| {
                        for option in &self.option_list {
                            let option = option.clone();
                            let settings = settings.clone();
                            let on_model = on_model.clone();
                            menu = menu.item(
                                PopupMenuItem::new(model_label(&option.model))
                                    .checked(option.model == settings.model)
                                    .on_click(move |_, _, cx| {
                                        let mut next = settings.clone();
                                        next.model = option.model.clone();
                                        if next.reasoning_effort.as_ref().is_some_and(|effort| {
                                            !option.reasoning_effort_list.contains(effort)
                                        }) {
                                            next.reasoning_effort = None;
                                        }
                                        on_model(next, cx);
                                    }),
                            );
                        }
                        let on_refresh = self.on_refresh.clone();
                        menu.separator().item(
                            PopupMenuItem::new("Refresh models")
                                .on_click(move |_, _, cx| on_refresh(cx)),
                        )
                    }),
            )
            .child(
                Button::new("reasoning")
                    .ghost()
                    .compact()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(effort_icon)
                            .child(effort_text),
                    )
                    .accessibility_label("Reasoning effort")
                    .tooltip("Reasoning effort for the next turn")
                    .disabled(disabled || model.is_empty())
                    .dropdown_menu(move |mut menu, _, _| {
                        let mut choice_list = vec![None];
                        choice_list.extend(effort_list.iter().cloned().map(Some));
                        for effort in choice_list {
                            let on_effort = on_effort.clone();
                            let model = model.clone();
                            menu = menu.item(
                                PopupMenuItem::new(effort_label(effort.as_deref())).on_click(
                                    move |_, _, cx| {
                                        on_effort(
                                            ModelSettings {
                                                model: model.clone(),
                                                reasoning_effort: effort.clone(),
                                            },
                                            cx,
                                        );
                                    },
                                ),
                            );
                        }
                        menu
                    }),
            );

        let input_box = div()
            .flex()
            .flex_col()
            .gap(px(20.))
            .rounded(px(22.))
            .bg(rgb(0x28292c))
            .border_1()
            .border_color(rgb(0x393a3d))
            .child(
                // The textarea already adds 10px horizontally and 8px vertically.
                div().px(px(2.)).pt(px(4.)).min_w_0().child(
                    Textarea::new(&self.prompt)
                        .appearance(false)
                        .bordered(false),
                ),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .h(px(28.))
                    .mx(px(8.))
                    .mb(px(8.))
                    .child(
                        Button::new("send-stop")
                            .primary()
                            .rounded_full()
                            .size(px(28.))
                            .ml_2()
                            .icon(if self.stopping {
                                IconName::Square
                            } else {
                                IconName::ArrowUp
                            })
                            .accessibility_label(if self.stopping { "Stop" } else { "Send" })
                            .tooltip(if self.stopping {
                                "Stop"
                            } else {
                                "Send · Enter"
                            })
                            .disabled(!self.stopping && self.cannot_send)
                            .on_click(move |_, window, cx| (self.on_submit)(window, cx)),
                    ),
            );

        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(controls)
            .child(input_box)
    }
}

fn model_label(model: &str) -> String {
    let mut part_list = model.split('-');
    let first = part_list.next().unwrap_or_default();
    let mut label = if first == "gpt" {
        "GPT".into()
    } else {
        first.to_string()
    };
    for part in part_list {
        let separator = if part.starts_with(|c: char| c.is_ascii_digit()) {
            "-"
        } else {
            " "
        };
        label.push_str(separator);
        let mut chars = part.chars();
        if let Some(first) = chars.next() {
            label.extend(first.to_uppercase());
        }
        label.extend(chars);
    }
    label
}

fn effort_label(effort: Option<&str>) -> String {
    match effort {
        None => "Automatic".into(),
        Some("xhigh") => "Extra high".into(),
        Some(value) => {
            let mut chars = value.chars();
            let mut label = String::new();
            if let Some(first) = chars.next() {
                label.extend(first.to_uppercase());
            }
            label.extend(chars);
            label
        }
    }
}

type SettingsHandler = dyn Fn(ModelSettings, &mut App);
type SubmitHandler = dyn Fn(&mut Window, &mut App);

fn effort_signal(effort: Option<&str>) -> Div {
    let active_count = match effort {
        Some("low") => 1,
        Some("medium") => 2,
        Some("high") => 3,
        Some("xhigh") => 4,
        Some("max" | "ultra") => 5,
        _ => 0,
    };
    div()
        .flex()
        .items_end()
        .gap(px(2.))
        .h(px(14.))
        .children((0..5).map(|index| {
            div()
                .w(px(2.))
                .h(px(4. + index as f32 * 2.5))
                .rounded(px(1.))
                .bg(if index < active_count {
                    rgb(0xd8dadd)
                } else {
                    rgb(0x55575c)
                })
        }))
}
