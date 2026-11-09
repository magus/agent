use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ToolRequest {
    #[serde(default)]
    pub owner: String,
    pub id: String,
    pub call_id: String,
    pub host: String,
    pub tool: Value,
    pub status: String,
    pub finished: bool,
}
