use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct ModelOption {
    pub model: String,
    pub reasoning_effort_list: Vec<String>,
}
