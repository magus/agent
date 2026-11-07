use gpui_kit::MenuItem;

gpui_kit::actions!(chatgpt, [Open]);

pub(crate) fn menu_item_list() -> Vec<MenuItem> {
    vec![MenuItem::action("ChatGPT", Open)]
}
