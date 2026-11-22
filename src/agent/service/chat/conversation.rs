use crate::protocol::{Entry, Target};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;

#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct Conversation {
    #[serde(default)]
    pub id: String,
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,
    pub input_list: Vec<Value>,
    pub entry_list: Vec<Entry>,
    pub status: String,
}

impl Conversation {
    pub fn load(directory: &Path) -> io::Result<Self> {
        fs::create_dir_all(directory)?;
        fs::set_permissions(directory, fs::Permissions::from_mode(0o700))?;
        let path = directory.join("conversation.json");
        let mut conversation: Self = match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(error) => return Err(error),
        };
        if conversation.id.is_empty() {
            conversation.id = uuid::Uuid::new_v4().to_string();
        }
        if conversation.status == "Running" {
            conversation.status = "Interrupted. Retrieve pending results before continuing.".into();
        }
        conversation.restore_search_entries();
        conversation.restore_search_images();
        conversation.finish_unsubmitted_calls();
        Ok(conversation)
    }

    fn restore_search_entries(&mut self) {
        let saved_queries = self
            .input_list
            .iter()
            .filter_map(super::search::query_for_call);
        let visible_queries = self
            .entry_list
            .iter()
            .filter(|entry| entry.role == "Search")
            .map(|entry| &entry.text);
        if saved_queries.eq(visible_queries.cloned()) {
            return;
        }

        self.entry_list.retain(|entry| entry.role != "Search");
        let mut cursor = 0;
        for item in &self.input_list {
            if let Some(query) = super::search::query_for_call(item) {
                self.entry_list.insert(cursor, Entry::text("Search", query));
                cursor += 1;
                continue;
            }
            let role = if item["role"] == "user" && item["content"].is_string() {
                "You"
            } else if item["type"] == "message"
                && item["content"].as_array().is_some_and(|parts| {
                    parts
                        .iter()
                        .any(|part| part["text"].as_str().is_some_and(|text| !text.is_empty()))
                })
            {
                "Agent"
            } else if item["type"] == "function_call" {
                "Tool"
            } else {
                continue;
            };
            let position = self.entry_list[cursor..].iter().position(|entry| {
                if role == "Tool" {
                    return entry
                        .request
                        .as_ref()
                        .is_some_and(|request| item["call_id"] == request.call_id);
                }
                entry.role == role
            });
            if let Some(position) = position {
                cursor += position + 1;
            }
        }
    }

    fn restore_search_images(&mut self) {
        let mut rows = self
            .entry_list
            .iter_mut()
            .filter(|entry| entry.role == "Search");
        let mut current = None;
        for item in &self.input_list {
            if super::search::query_for_call(item).is_some() {
                current = rows.next();
            } else if item["role"] == "user" {
                current = None;
            } else if item["type"] == "web_search_call"
                && let Some(entry) = current.as_deref_mut()
            {
                super::search::append_images(entry, item);
            } else if item["type"] == "function_call_output"
                && let Some(entry) = current.as_deref_mut()
                && let Some(result) = item["output"]
                    .as_str()
                    .and_then(|text| serde_json::from_str::<Value>(text).ok())
                && let Some(searches) = result["searches"].as_array()
            {
                for search in searches {
                    super::search::append_images(entry, search);
                }
            }
        }
    }

    pub fn save(&self, directory: &Path) -> io::Result<()> {
        let temporary = directory.join("conversation.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary)?;
        file.write_all(&serde_json::to_vec(self)?)?;
        file.sync_all()?;
        fs::rename(temporary, directory.join("conversation.json"))?;
        fs::File::open(directory)?.sync_all()?;
        Ok(())
    }

    pub fn finish_unsubmitted_calls(&mut self) {
        let call_id_list: Vec<String> = self
            .input_list
            .iter()
            .filter(|item| item["type"] == "function_call")
            .filter_map(|item| item["call_id"].as_str().map(str::to_owned))
            .collect();
        for call_id in call_id_list {
            let has_output = self
                .input_list
                .iter()
                .any(|item| item["type"] == "function_call_output" && item["call_id"] == call_id);
            let has_request = self.entry_list.iter().any(|entry| {
                entry
                    .request
                    .as_ref()
                    .is_some_and(|request| request.call_id == call_id)
            });
            if !has_output && !has_request {
                self.input_list.push(serde_json::json!({"type":"function_call_output", "call_id":call_id, "output":"Not submitted. The task stopped before this tool ran."}));
            }
        }
    }

    pub fn has_message(&self, id: &str, sender: &Target, prompt: &str) -> io::Result<bool> {
        let Some(entry) = self
            .entry_list
            .iter()
            .find(|entry| entry.message_id.as_deref() == Some(id))
        else {
            return Ok(false);
        };
        if entry
            .sender
            .as_ref()
            .is_none_or(|original| original.id != sender.id)
            || entry.text != prompt
        {
            return Err(io::Error::other(
                "Message ID belongs to different content or sender.",
            ));
        }
        Ok(true)
    }

    pub fn pending(&self) -> bool {
        self.entry_list.iter().any(|entry| {
            entry
                .request
                .as_ref()
                .is_some_and(|request| !request.finished)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::ToolRequest;
    use serde_json::json;

    #[test]
    fn historical_search_is_visible_once_between_question_and_answer() {
        let mut conversation = Conversation {
            input_list: vec![
                json!({"role":"user","content":"Find a quote"}),
                json!({"type":"function_call","name":"search","arguments":r#"{"query":"AAPL quote"}"#}),
                json!({"type":"message","content":[{"text":"Quote found"}]}),
            ],
            entry_list: vec![
                Entry::text("You", "Find a quote"),
                Entry::text("Agent", "Quote found"),
            ],
            ..Conversation::default()
        };
        conversation.restore_search_entries();
        conversation.restore_search_entries();
        let rows: Vec<_> = conversation
            .entry_list
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        assert_eq!(rows, ["Find a quote", "AAPL quote", "Quote found"]);
    }

    #[test]
    fn legacy_preferred_device_is_not_retained() {
        let mut saved = serde_json::to_value(Conversation::default()).unwrap();
        saved["host"] = json!("revoked-device");

        let restored: Conversation = serde_json::from_value(saved).unwrap();
        let saved = serde_json::to_value(restored).unwrap();

        assert!(saved.get("host").is_none());
    }

    #[test]
    fn message_receipts_survive_restart_and_reject_changed_sender_or_content() {
        let directory =
            std::env::temp_dir().join(format!("agent-receipt-{}", uuid::Uuid::new_v4()));
        let mut conversation = Conversation::load(&directory).unwrap();
        let sender = Target {
            id: "macbook".into(),
            name: "MacBook".into(),
        };
        let mut entry = Entry::text("You", "Run hostname here");
        entry.message_id = Some("request-id".into());
        entry.sender = Some(sender.clone());
        conversation.entry_list.push(entry);
        conversation.save(&directory).unwrap();

        let restored = Conversation::load(&directory).unwrap();
        assert!(
            restored
                .has_message("request-id", &sender, "Run hostname here")
                .unwrap()
        );
        assert!(
            !restored
                .has_message("new-id", &sender, "Run hostname here")
                .unwrap()
        );
        assert!(
            restored
                .has_message("request-id", &sender, "Different action")
                .is_err()
        );
        let other = Target {
            id: "mini".into(),
            name: "mini".into(),
        };
        assert!(
            restored
                .has_message("request-id", &other, "Run hostname here")
                .is_err()
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn interruption_keeps_submitted_requests_and_closes_only_unsubmitted_calls() {
        let directory =
            std::env::temp_dir().join(format!("agent-history-{}", uuid::Uuid::new_v4()));
        let mut conversation = Conversation::load(&directory).unwrap();
        conversation.status = "Running".into();
        conversation.input_list = vec![
            json!({"type":"function_call", "call_id":"submitted"}),
            json!({"type":"function_call", "call_id":"not-submitted"}),
        ];
        let mut entry = Entry::text("Tool", "A submitted command");
        entry.request = Some(ToolRequest {
            owner: "source".into(),
            id: "same-request-id".into(),
            call_id: "submitted".into(),
            host: "mini".into(),
            tool: json!({}),
            status: "running".into(),
            finished: false,
        });
        conversation.entry_list.push(entry);
        conversation.save(&directory).unwrap();

        let restored = Conversation::load(&directory).unwrap();
        assert!(restored.pending());
        assert!(restored.status.starts_with("Interrupted"));
        assert_eq!(
            restored.entry_list[0].request.as_ref().unwrap().id,
            "same-request-id"
        );
        assert_eq!(restored.input_list.len(), 3);
        assert_eq!(restored.input_list[2]["call_id"], "not-submitted");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
