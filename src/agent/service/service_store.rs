use crate::protocol::ConversationRecord;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub(crate) struct ServiceStore {
    directory: PathBuf,
    state: Mutex<State>,
}

impl ServiceStore {
    pub fn load(directory: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&directory)?;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;

        let state = match fs::read(directory.join("conversations.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => State::default(),
            Err(error) => return Err(error),
        };

        Ok(Self {
            directory,
            state: Mutex::new(state),
        })
    }

    pub fn conversation_list(&self) -> Vec<ConversationRecord> {
        self.state
            .lock()
            .unwrap()
            .conversation_by_id
            .values()
            .cloned()
            .collect()
    }

    pub fn get(&self, id: &str) -> io::Result<ConversationRecord> {
        self.state
            .lock()
            .unwrap()
            .conversation_by_id
            .get(id)
            .cloned()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Conversation not found"))
    }

    pub fn create(&self, sender: &str, id: &str, title: &str) -> io::Result<ConversationRecord> {
        uuid::Uuid::parse_str(id).map_err(io::Error::other)?;
        if title.trim().is_empty() || title.len() > 200 {
            return Err(io::Error::other(
                "Conversation title must contain 1–200 bytes.",
            ));
        }

        let mut state = self.state.lock().unwrap();
        if let Some(existing) = state.conversation_by_id.get(id) {
            if existing.creator != sender || existing.title != title {
                return Err(io::Error::other(
                    "Conversation ID belongs to another creation request.",
                ));
            }
            return Ok(existing.clone());
        }

        let allowed_target_list = state
            .default_target_list_by_device
            .get(sender)
            .cloned()
            .unwrap_or_default();
        let model_settings = state
            .model_settings_by_device
            .get(sender)
            .cloned()
            .unwrap_or_default();
        let record = ConversationRecord {
            model_settings,
            id: id.into(),
            title: title.into(),
            creator: sender.into(),
            allowed_target_list,
        };
        let mut updated = state.clone();
        updated.conversation_by_id.insert(id.into(), record.clone());
        self.save(&updated)?;
        *state = updated;
        Ok(record)
    }

    pub fn default_target_list(&self, device: &str) -> Vec<String> {
        self.state
            .lock()
            .unwrap()
            .default_target_list_by_device
            .get(device)
            .cloned()
            .unwrap_or_default()
    }

    pub fn model_settings(&self, device: &str) -> crate::protocol::ModelSettings {
        self.state
            .lock()
            .unwrap()
            .model_settings_by_device
            .get(device)
            .cloned()
            .unwrap_or_default()
    }

    pub fn set_model_settings(
        &self,
        device: &str,
        settings: crate::protocol::ModelSettings,
    ) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        let mut updated = state.clone();
        updated
            .model_settings_by_device
            .insert(device.into(), settings);
        self.save(&updated)?;
        *state = updated;
        Ok(())
    }

    pub fn set_default_targets(&self, device: &str, target_list: Vec<String>) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        let mut updated = state.clone();
        updated
            .default_target_list_by_device
            .insert(device.into(), unique_targets(target_list));
        self.save(&updated)?;
        *state = updated;
        Ok(())
    }

    pub fn set_allowed_targets(&self, id: &str, target_list: Vec<String>) -> io::Result<()> {
        let mut state = self.state.lock().unwrap();
        let mut updated = state.clone();
        let record = updated
            .conversation_by_id
            .get_mut(id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Conversation not found"))?;
        record.allowed_target_list = unique_targets(target_list);
        self.save(&updated)?;
        *state = updated;
        Ok(())
    }

    pub fn with_target_access<T>(
        &self,
        id: &str,
        target: &str,
        dispatch: impl FnOnce() -> io::Result<T>,
    ) -> io::Result<T> {
        let state = self.state.lock().unwrap();
        let record = state
            .conversation_by_id
            .get(id)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Conversation not found"))?;
        if !record
            .allowed_target_list
            .iter()
            .any(|allowed| allowed == target)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "This device is not enabled for this conversation.",
            ));
        }

        // Keep the grant stable until dispatch returns. Revocation controls later submissions.
        dispatch()
    }

    pub fn conversation_dir(&self, id: &str) -> io::Result<PathBuf> {
        self.get(id)?;
        uuid::Uuid::parse_str(id).map_err(io::Error::other)?;
        let path = self.directory.join("conversations").join(id);
        fs::create_dir_all(&path)?;
        Ok(path)
    }

    fn save(&self, state: &State) -> io::Result<()> {
        save_json(&self.directory.join("conversations.json"), state)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    model_settings_by_device: BTreeMap<String, crate::protocol::ModelSettings>,
    conversation_by_id: BTreeMap<String, ConversationRecord>,
    default_target_list_by_device: BTreeMap<String, Vec<String>>,
}

fn unique_targets(mut target_list: Vec<String>) -> Vec<String> {
    target_list.sort();
    target_list.dedup();
    target_list
}

fn save_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let temporary = path.with_extension("tmp");
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(&temporary)?;
    file.write_all(&serde_json::to_vec(value)?)?;
    file.sync_all()?;
    fs::rename(temporary, path)?;
    fs::File::open(path.parent().unwrap())?.sync_all()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_defaults_survive_restart_and_do_not_change_existing_conversations() {
        let directory =
            std::env::temp_dir().join(format!("agent-defaults-{}", uuid::Uuid::new_v4()));
        let store = ServiceStore::load(directory.clone()).unwrap();
        let original_id = uuid::Uuid::new_v4().to_string();
        store.create("macbook", &original_id, "Original").unwrap();
        let settings = crate::protocol::ModelSettings {
            model: "test-model".into(),
            reasoning_effort: Some("high".into()),
        };
        store
            .set_model_settings("macbook", settings.clone())
            .unwrap();
        drop(store);

        let store = ServiceStore::load(directory.clone()).unwrap();
        assert!(store.model_settings("macbook") == settings);
        assert!(store.model_settings("mini").is_default());
        assert!(
            store
                .create("macbook", &original_id, "Original")
                .unwrap()
                .model_settings
                .is_default()
        );
        let new_id = uuid::Uuid::new_v4().to_string();
        assert!(
            store
                .create("macbook", &new_id, "New")
                .unwrap()
                .model_settings
                == settings
        );
        store
            .set_model_settings("macbook", Default::default())
            .unwrap();
        assert!(
            store
                .create("macbook", &new_id, "New")
                .unwrap()
                .model_settings
                == settings
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn defaults_are_copied_and_retries_preserve_permissions_after_restart() {
        let directory =
            std::env::temp_dir().join(format!("agent-service-{}", uuid::Uuid::new_v4()));
        let store = ServiceStore::load(directory.clone()).unwrap();
        store
            .set_default_targets("macbook", vec!["macbook".into(), "mini".into()])
            .unwrap();
        store
            .set_default_targets("mini", vec!["mini".into()])
            .unwrap();
        let first_id = uuid::Uuid::new_v4().to_string();
        store.create("mini", &first_id, "Background task").unwrap();
        assert!(
            store
                .with_target_access(&first_id, "macbook", || Ok(()))
                .is_err()
        );
        assert!(
            store
                .with_target_access(&first_id, "mini", || Ok(()))
                .is_ok()
        );

        store
            .set_default_targets("mini", vec!["mini".into(), "macbook".into()])
            .unwrap();
        let original = store.create("mini", &first_id, "Background task").unwrap();
        assert_eq!(original.allowed_target_list, vec!["mini"]);
        assert!(
            store
                .create("macbook", &first_id, "Background task")
                .is_err()
        );
        drop(store);

        let restored = ServiceStore::load(directory.clone()).unwrap();
        assert_eq!(
            restored.get(&first_id).unwrap().allowed_target_list,
            vec!["mini"]
        );
        restored
            .set_allowed_targets(&first_id, vec!["mini".into(), "macbook".into()])
            .unwrap();
        assert!(
            restored
                .with_target_access(&first_id, "macbook", || Ok(()))
                .is_ok()
        );
        restored
            .set_allowed_targets(&first_id, vec!["mini".into()])
            .unwrap();
        assert!(
            restored
                .with_target_access::<()>(&first_id, "macbook", || panic!("revoked dispatch ran"))
                .is_err()
        );
        assert_eq!(restored.conversation_list().len(), 1);
        assert!(restored.conversation_dir(&first_id).unwrap().is_dir());
        assert_eq!(restored.default_target_list("mini").len(), 2);
        fs::remove_dir_all(directory).unwrap();
    }
}
