use crate::protocol::LoginRequest;
use crate::service::chatgpt::{ChatGptClient, PendingLogin};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

pub(super) struct CentralLogin {
    attempt_by_state: Mutex<HashMap<String, Arc<Attempt>>>,
}

impl CentralLogin {
    pub fn new() -> Self {
        Self {
            attempt_by_state: Mutex::new(HashMap::new()),
        }
    }

    pub fn handle(&self, client: &ChatGptClient, owner: &str, request: Value) -> io::Result<Value> {
        let request: LoginRequest = serde_json::from_value(request)?;
        match request {
            LoginRequest::Registration => Ok(json!({"client_id":client.login_client_id()?})),
            LoginRequest::Begin {
                state,
                redirect_uri,
            } => {
                let mut attempt_by_state = self.attempt_by_state.lock().unwrap();
                if attempt_by_state.contains_key(&state) {
                    return Err(io::Error::other("This login attempt already exists."));
                }
                let (pending, url) = client.begin_login(redirect_uri, &state)?;
                for attempt in attempt_by_state
                    .values()
                    .filter(|attempt| attempt.owner == owner)
                {
                    attempt.cancelled.store(true, Ordering::SeqCst);
                }
                attempt_by_state.insert(
                    state,
                    Arc::new(Attempt {
                        owner: owner.into(),
                        pending: Mutex::new(Some(pending)),
                        cancelled: AtomicBool::new(false),
                        result: Mutex::new(None),
                    }),
                );
                Ok(json!({"url":url}))
            }
            LoginRequest::Complete {
                state,
                code,
                client_id,
            } => {
                let attempt = self.attempt(owner, &state)?;
                let mut result = attempt.result.lock().unwrap();
                if let Some(saved) = &*result {
                    return saved
                        .clone()
                        .map(|label| json!({"account":label}))
                        .map_err(io::Error::other);
                }
                let pending = attempt
                    .pending
                    .lock()
                    .unwrap()
                    .take()
                    .ok_or_else(|| io::Error::other("Login attempt was already consumed."))?;
                let completed = client
                    .complete_login(pending, &code, &client_id, &attempt.cancelled)
                    .map_err(|error| error.to_string());
                *result = Some(completed.clone());
                completed
                    .map(|label| json!({"account":label}))
                    .map_err(io::Error::other)
            }
            LoginRequest::Cancel { state } => {
                let attempt = self.attempt(owner, &state)?;
                attempt.cancelled.store(true, Ordering::SeqCst);
                Ok(json!({"cancelled":true}))
            }
        }
    }

    fn attempt(&self, owner: &str, state: &str) -> io::Result<Arc<Attempt>> {
        let attempt = self
            .attempt_by_state
            .lock()
            .unwrap()
            .get(state)
            .cloned()
            .ok_or_else(|| io::Error::other("Unknown login attempt. Start login again."))?;
        if attempt.owner != owner {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Login belongs to another device.",
            ));
        }
        Ok(attempt)
    }
}

struct Attempt {
    owner: String,
    pending: Mutex<Option<PendingLogin>>,
    cancelled: AtomicBool,
    result: Mutex<Option<Result<String, String>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_device_that_started_login_can_complete_or_cancel_it() {
        let login = CentralLogin::new();
        login.attempt_by_state.lock().unwrap().insert(
            "state".into(),
            Arc::new(Attempt {
                owner: "macbook".into(),
                pending: Mutex::new(None),
                cancelled: AtomicBool::new(false),
                result: Mutex::new(Some(Ok("Signed in".into()))),
            }),
        );

        assert!(login.attempt("macbook", "state").is_ok());
        let denied = login.attempt("mini", "state").err().unwrap();
        assert_eq!(denied.kind(), io::ErrorKind::PermissionDenied);
        assert!(login.attempt("macbook", "unknown").is_err());
    }
}
