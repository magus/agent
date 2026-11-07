use gpui_kit::MenuItem;

gpui_kit::actions!(desktop_service, [OpenDesktopPermissions]);

pub(crate) fn menu_item_list() -> Vec<MenuItem> {
    vec![MenuItem::action(
        "Desktop permissions…",
        OpenDesktopPermissions,
    )]
}
