use super::{ConversationRecord, ConversationView, Target};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(crate) struct ConversationResponse {
    pub conversation: ConversationView,
    pub record: ConversationRecord,
    pub running: bool,
    pub revision: u64,
    pub image_by_id: HashMap<String, String>,
    pub target_list: Vec<Target>,
}
