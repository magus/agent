use crate::app::args::Args;
use crate::app::chatgpt;
use crate::app::desktop_service;
use gpui_kit::component::Theme;
use gpui_kit::{KeyBinding, Menu, MenuItem};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

gpui_kit::actions!(agent, [Quit, Reopen]);

pub(crate) struct App;

impl App {
    pub(crate) fn run(args: Args) -> io::Result<()> {
        let application = gpui_kit::application().with_assets(gpui_kit::assets::AllAssets);
        application.on_reopen(|cx| cx.dispatch_action(&Reopen));

        let startup_error = Rc::new(RefCell::new(None));
        let error_result = startup_error.clone();
        application.run(move |cx| {
            gpui_kit::init(cx);
            Theme::sync_system_appearance(None, cx);

            cx.on_action(|_: &Quit, cx| cx.quit());
            cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);

            if let Err(error) = Self::init(args, cx) {
                *error_result.borrow_mut() = Some(error);
                cx.quit();
            }
        });

        match startup_error.take() {
            Some(error) => Err(error),
            None => Ok(()),
        }
    }

    fn init(args: Args, cx: &mut gpui_kit::App) -> io::Result<()> {
        desktop_service::init(args, cx)?;
        let devices = crate::app::devices::init(args, cx)?;
        let chat = crate::app::chat::init(args, devices, cx)?;
        chatgpt::init(args, chat, cx)?;

        let mut menu_item_list = Vec::new();

        menu_item_list.extend(crate::app::chat::menu_item_list());
        menu_item_list.extend(desktop_service::menu_item_list());
        menu_item_list.extend(chatgpt::menu_item_list());
        menu_item_list.extend(crate::app::devices::menu_item_list());
        menu_item_list.extend([MenuItem::separator(), MenuItem::action("Quit Agent", Quit)]);

        cx.set_menus([Menu::new("Agent").items(menu_item_list)]);
        Ok(())
    }
}
