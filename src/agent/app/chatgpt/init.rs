use super::chatgpt_client::ChatGptClient;
use super::chatgpt_setup::ChatGptSetup;
use super::menu_item_list::Open;
use crate::app::args::Args;
use gpui_kit::App;
use std::io;
use std::sync::Arc;

pub(crate) fn init(args: Args, cx: &mut App) -> io::Result<()> {
    let runtime_dir = crate::app::core::runtime_dir()?;
    let client = Arc::new(ChatGptClient::new(&runtime_dir)?);

    let setup_client = client.clone();
    cx.on_action(move |_: &Open, cx| ChatGptSetup::open(setup_client.clone(), cx));

    if args == Args::ChatGpt {
        ChatGptSetup::open(client, cx);
    }

    Ok(())
}
