use gpui_kit::MenuItem;

gpui_kit::actions!(devices, [OpenDevices]);

pub(crate) fn menu_item_list() -> Vec<MenuItem> {
    vec![MenuItem::action("Devices", OpenDevices)]
}
