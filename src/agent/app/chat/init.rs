use super::chat_client::ChatClient;
use super::chat_window::ChatWindow;
use crate::app::Reopen;
use crate::app::args::Args;
use crate::devices::ToolConnection;
use std::io;
use std::sync::Arc;

use super::conversation_settings::ConversationSettings;
use super::menu_item_list::{OpenChat, OpenConversationSettings};

pub(crate) fn init(
    args: Args,
    devices: Arc<ToolConnection>,
    cx: &mut gpui_kit::App,
) -> io::Result<Arc<ChatClient>> {
    let directory = crate::platform::runtime_dir()?.join("client");
    let agent = ChatClient::load(devices, directory)?;
    let reopen_agent = agent.clone();
    cx.on_action(move |_: &Reopen, cx| ChatWindow::open(reopen_agent.clone(), cx));
    let open_agent = agent.clone();
    cx.on_action(move |_: &OpenChat, cx| ChatWindow::open(open_agent.clone(), cx));
    let settings_agent = agent.clone();
    cx.on_action(move |_: &OpenConversationSettings, cx| {
        ConversationSettings::open(settings_agent.clone(), cx);
    });

    if args == Args::Chat {
        ChatWindow::open(agent.clone(), cx);
    }
    Ok(agent)
}
