use super::chatgpt_setup::ChatGptSetup;
use super::menu_item_list::Open;
use super::remote_account::RemoteAccount;
use crate::app::args::Args;
use crate::app::chat::ChatClient;
use gpui_kit::App;
use std::io;
use std::sync::Arc;

pub(crate) fn init(args: Args, chat: Arc<ChatClient>, cx: &mut App) -> io::Result<()> {
    let client = Arc::new(RemoteAccount::new(chat));

    let setup_client = client.clone();
    cx.on_action(move |_: &Open, cx| ChatGptSetup::open(setup_client.clone(), cx));

    if args == Args::ChatGpt {
        ChatGptSetup::open(client.clone(), cx);
    }

    Ok(())
}
