use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ConversationRecord {
    pub id: String,
    pub title: String,
    pub creator: String,
    #[serde(default, skip_serializing_if = "super::ModelSettings::is_default")]
    pub model_settings: super::ModelSettings,
    pub allowed_target_list: Vec<String>,
}
