use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum ServiceRequest {
    List,
    Login {
        request: Value,
    },
    SearchSources {
        id: String,
        entry_index: usize,
    },
    ConversationJson {
        id: String,
        events: bool,
        offset: u64,
        snapshot: Option<(u64, u64)>,
    },
    CheckModel,
    SignOut,
    Models,
    ModelOptions,
    SetModelDefaults {
        settings: super::ModelSettings,
    },
    SetConversationModel {
        id: String,
        settings: super::ModelSettings,
    },
    Account,
    Create {
        id: String,
        title: String,
    },
    Get {
        id: String,
        #[serde(default)]
        known_image_list: Vec<String>,
    },
    Send {
        id: String,
        message_id: String,
        prompt: String,
        model: String,
        reasoning_effort: Option<String>,
    },
    Stop {
        id: String,
    },
    Retrieve {
        id: String,
    },
    SetDefaults {
        target_list: Vec<String>,
    },
    SetTargets {
        id: String,
        target_list: Vec<String>,
    },
}
