use crate::protocol::Entry;
use gpui_kit::base::SelectableText;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, IconName};
use gpui_kit::{prelude::*, *};
use std::rc::Rc;

#[derive(IntoElement)]
pub(super) struct ToolRow {
    pub entry: Entry,
    pub device_name: String,
    pub expanded: bool,
    pub on_toggle: Rc<dyn Fn(&mut App)>,
}

impl RenderOnce for ToolRow {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let request = self
            .entry
            .request
            .as_ref()
            .expect("tool row requires a request");
        let id = request.id.clone();
        let summary = tool_summary(&request.tool);
        let (status, icon, color) = match request.status.as_str() {
            "completed" => ("Done", IconName::Check, rgb(0x999ca4)),
            "failed" => ("Failed", IconName::CircleAlert, rgb(0xe59a90)),
            "cancelled" => ("Stopped", IconName::Square, rgb(0xb6a68b)),
            "interrupted" => ("Interrupted", IconName::CircleAlert, rgb(0xb6a68b)),
            _ => ("Running", IconName::Loader, rgb(0xaebed7)),
        };
        let arguments = serde_json::to_string_pretty(&request.tool).unwrap_or_default();
        let mut row = div()
            .id(SharedString::from(format!("tool-{id}")))
            .w_full()
            .min_w_0()
            .flex()
            .flex_col()
            .rounded_lg()
            .child(
                Button::new(SharedString::from(format!("toggle-{id}")))
                    .ghost()
                    .accessibility_label(format!("{summary} · {} · {status}", self.device_name))
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .items_center()
                            .gap_2()
                            .h(px(32.))
                            .px_3()
                            .rounded_md()
                            .cursor_pointer()
                            .hover(|style| style.bg(rgb(0x242529)))
                            .text_sm()
                            .text_color(rgb(0xa5a7ae))
                            .child(Icon::new(IconName::SquareTerminal).size_4().flex_shrink_0())
                            .child(div().flex_1().min_w_0().truncate().child(summary))
                            .child(div().max_w(px(160.)).truncate().child(self.device_name))
                            .child(
                                div()
                                    .w(px(94.))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .text_color(color)
                                    .child(Icon::new(icon).size_3())
                                    .child(status),
                            )
                            .child(
                                Icon::new(if self.expanded {
                                    IconName::ChevronDown
                                } else {
                                    IconName::ChevronRight
                                })
                                .size_3()
                                .flex_shrink_0(),
                            ),
                    )
                    .on_click(move |_, _, cx| (self.on_toggle)(cx)),
            );
        if self.expanded {
            row = row.child(
                div()
                    .min_w_0()
                    .ml_3()
                    .pl_4()
                    .py_3()
                    .border_l_1()
                    .border_color(rgb(0x393b40))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .text_sm()
                    .child(SelectableText::new(
                        SharedString::from(format!("arguments-{id}")),
                        arguments,
                    ))
                    .child(SelectableText::new(
                        SharedString::from(format!("output-{id}")),
                        self.entry.text,
                    )),
            );
            if let Some(path) = self.entry.image {
                let open_path = path.clone();
                row = row.child(
                    div()
                        .p_3()
                        .flex()
                        .flex_col()
                        .gap_2()
                        .child(
                            img(path)
                                .w_full()
                                .h(px(300.))
                                .object_fit(ObjectFit::Contain),
                        )
                        .child(
                            Button::new(SharedString::from(format!("image-{id}")))
                                .ghost()
                                .compact()
                                .label("Open screenshot")
                                .icon(IconName::ExternalLink)
                                .on_click(move |_, _, cx| {
                                    if let Ok(url) = url::Url::from_file_path(&open_path) {
                                        cx.open_url(url.as_str());
                                    }
                                }),
                        ),
                );
            }
        }
        row
    }
}

fn tool_summary(tool: &serde_json::Value) -> String {
    if tool["tool"] == "shell" {
        let command = tool["command"].as_str().unwrap_or("Shell command");
        return command.split_whitespace().collect::<Vec<_>>().join(" ");
    }
    let action = &tool["action"];
    let operation = action["operation"].as_str().unwrap_or("action");
    match operation {
        "capture" => "Capture desktop".into(),
        "click" => "Click desktop".into(),
        "drag" => "Drag on desktop".into(),
        "scroll" => "Scroll desktop".into(),
        "type" | "text" => "Type text".into(),
        "key" | "keys" => "Press keys".into(),
        _ => format!("Computer · {operation}"),
    }
}
