use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum LoginRequest {
    Registration,
    Begin {
        state: String,
        redirect_uri: String,
    },
    Complete {
        state: String,
        code: String,
        client_id: String,
    },
    Cancel {
        state: String,
    },
}
