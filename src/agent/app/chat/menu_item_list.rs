gpui_kit::actions!(chat, [OpenChat, OpenConversationSettings]);

pub(crate) fn menu_item_list() -> Vec<gpui_kit::MenuItem> {
    vec![
        gpui_kit::MenuItem::action("Chat", OpenChat),
        gpui_kit::MenuItem::action("Conversation settings…", OpenConversationSettings),
    ]
}
