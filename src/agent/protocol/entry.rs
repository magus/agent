use super::{Target, ToolRequest};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Entry {
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub sender: Option<Target>,
    pub role: String,
    pub text: String,
    pub image: Option<PathBuf>,
    pub request: Option<ToolRequest>,
}

impl Entry {
    pub fn text(role: &str, text: impl Into<String>) -> Self {
        Self {
            sender: None,
            message_id: None,
            role: role.into(),
            text: text.into(),
            image: None,
            request: None,
        }
    }
}
