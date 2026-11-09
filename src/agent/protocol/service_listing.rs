use super::{ConversationRecord, Target};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct ServiceListing {
    pub conversation_list: Vec<ConversationRecord>,
    pub target_list: Vec<Target>,
    pub default_target_list: Vec<String>,
    #[serde(default, skip_serializing_if = "super::ModelSettings::is_default")]
    pub model_settings: super::ModelSettings,
    pub service: Target,
}
