use gpui_kit::base::SelectableText;
use gpui_kit::component::text::TextView;
use gpui_kit::{prelude::*, *};

pub(super) fn message_bubble(index: usize, text: String, user: bool) -> Div {
    let content = div()
        .min_w_0()
        .max_w(relative(0.86))
        .px_4()
        .py_3()
        .rounded(px(18.));
    let content = if user {
        content
            .bg(rgb(0x243b58))
            .child(SelectableText::new(("message", index), text))
    } else {
        content.child(TextView::markdown(("message", index), text).selectable(true))
    };

    div()
        .w_full()
        .min_w_0()
        .flex()
        .when(user, |view| view.justify_end())
        .child(content)
}
