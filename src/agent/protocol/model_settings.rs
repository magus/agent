use serde::{Deserialize, Serialize};

#[derive(Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct ModelSettings {
    pub model: String,
    pub reasoning_effort: Option<String>,
}

impl ModelSettings {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
}
