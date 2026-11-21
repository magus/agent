use super::Entry;
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct ConversationView {
    #[serde(default)]
    pub id: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    pub entry_list: Vec<Entry>,
    pub status: String,
}
