use super::central_login::CentralLogin;
use super::service_store::ServiceStore;
use crate::devices::ToolConnection;
use crate::protocol::Target;
use crate::protocol::{ConversationResponse, ServiceListing, ServiceRequest};
use crate::service::chat::AgentLoop;
use crate::service::chatgpt::ChatGptClient;
use base64::Engine;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::io;
use std::sync::{Arc, Mutex};

pub(super) struct CentralService {
    login: CentralLogin,
    store: Arc<ServiceStore>,
    client: Arc<ChatGptClient>,
    devices: Arc<ToolConnection>,
    loop_by_id: Mutex<HashMap<String, Arc<AgentLoop>>>,
}

impl CentralService {
    pub fn new(
        store: Arc<ServiceStore>,
        client: Arc<ChatGptClient>,
        devices: Arc<ToolConnection>,
    ) -> Self {
        Self {
            login: CentralLogin::new(),
            store,
            client,
            devices,
            loop_by_id: Mutex::new(HashMap::new()),
        }
    }

    pub fn handle(&self, sender: Target, request: Value) -> io::Result<Value> {
        let request: ServiceRequest = serde_json::from_value(request)?;
        match request {
            ServiceRequest::List => serde_json::to_value(ServiceListing {
                conversation_list: self.store.conversation_list(),
                target_list: self.devices.target_list(),
                default_target_list: self.store.default_target_list(&sender.id),
                model_settings: self.store.model_settings(&sender.id),
                service: self.devices.local_device(),
            })
            .map_err(io::Error::other),
            ServiceRequest::Login { request } => {
                self.login.handle(&self.client, &sender.id, request)
            }
            ServiceRequest::SearchSources { id, entry_index } => {
                let directory = self.store.conversation_dir(&id)?;
                super::search_sources::load(&directory, entry_index)
            }
            ServiceRequest::ConversationJson {
                id,
                events,
                offset,
                snapshot,
            } => {
                let directory = self.store.conversation_dir(&id)?;
                super::conversation_json::chunk(&directory, events, offset, snapshot)
            }
            ServiceRequest::CheckModel => Ok(json!({"message":self.client.check_model()?})),
            ServiceRequest::SignOut => {
                for agent in self.loop_by_id.lock().unwrap().values() {
                    agent.stop();
                }
                Ok(json!({"message":self.client.sign_out()?}))
            }
            ServiceRequest::Models => Ok(json!({"model_list":self.client.model_list()?})),
            ServiceRequest::ModelOptions => {
                Ok(json!({"model_option_list": self.client.model_option_list()?}))
            }
            ServiceRequest::SetModelDefaults { settings } => {
                self.validate_model_settings(&settings)?;
                self.store.set_model_settings(&sender.id, settings)?;
                Ok(json!({"saved":true}))
            }
            ServiceRequest::SetConversationModel { id, settings } => {
                self.validate_model_settings(&settings)?;
                self.loop_for(&id)?.set_model_settings(settings)?;
                Ok(json!({"saved":true}))
            }
            ServiceRequest::Account => Ok(json!({"account":self.client.account()?})),
            ServiceRequest::Create { id, title } => {
                let record = self.store.create(&sender.id, &id, &title)?;
                self.store.conversation_dir(&id)?;
                Ok(serde_json::to_value(record)?)
            }
            ServiceRequest::Get {
                id,
                known_image_list,
            } => {
                let agent = self.loop_for(&id)?;
                let mut conversation = agent.view();
                let mut image_by_id = HashMap::new();
                for entry in &mut conversation.entry_list {
                    if let Some(path) = entry.image.take() {
                        let directory = self.store.conversation_dir(&id)?.canonicalize()?;
                        let path = path.canonicalize()?;
                        if !path.starts_with(directory) {
                            return Err(io::Error::other(
                                "Screenshot is outside the conversation directory.",
                            ));
                        }
                        let name = path
                            .file_name()
                            .and_then(|name| name.to_str())
                            .ok_or_else(|| io::Error::other("Invalid screenshot name"))?
                            .to_string();
                        if !known_image_list.contains(&name) {
                            image_by_id.insert(
                                name.clone(),
                                base64::engine::general_purpose::STANDARD
                                    .encode(std::fs::read(path)?),
                            );
                        }
                        entry.image = Some(name.into());
                    }
                }
                serde_json::to_value(ConversationResponse {
                    conversation,
                    record: self.store.get(&id)?,
                    running: agent.running(),
                    revision: agent.revision(),
                    image_by_id,
                    target_list: agent.allowed_target_list()?,
                })
                .map_err(io::Error::other)
            }
            ServiceRequest::Send {
                id,
                message_id,
                prompt,
                model,
                reasoning_effort,
            } => {
                let settings = crate::protocol::ModelSettings {
                    model,
                    reasoning_effort,
                };
                let result = self
                    .loop_for(&id)?
                    .start(sender, message_id, prompt, settings);
                match result {
                    Ok(()) => Ok(json!({"accepted":true})),
                    Err(error) => Ok(json!({"accepted":false, "error":error.to_string()})),
                }
            }
            ServiceRequest::Stop { id } => {
                self.loop_for(&id)?.stop();
                Ok(json!({"stopping":true}))
            }
            ServiceRequest::Retrieve { id } => {
                self.loop_for(&id)?.retrieve()?;
                Ok(json!({"retrieving":true}))
            }
            ServiceRequest::SetDefaults { target_list } => {
                self.validate_targets(&target_list)?;
                self.store.set_default_targets(&sender.id, target_list)?;
                Ok(json!({"saved":true}))
            }
            ServiceRequest::SetTargets { id, target_list } => {
                self.validate_targets(&target_list)?;
                self.store.set_allowed_targets(&id, target_list)?;
                Ok(json!({"saved":true}))
            }
        }
    }

    fn loop_for(&self, id: &str) -> io::Result<Arc<AgentLoop>> {
        let mut loop_by_id = self.loop_by_id.lock().unwrap();
        if let Some(agent) = loop_by_id.get(id) {
            return Ok(agent.clone());
        }
        let directory = self.store.conversation_dir(id)?;
        let agent = AgentLoop::load(
            self.client.clone(),
            self.devices.clone(),
            directory,
            self.store.clone(),
            id.into(),
        )?;
        loop_by_id.insert(id.into(), agent.clone());
        Ok(agent)
    }

    fn validate_model_settings(&self, settings: &crate::protocol::ModelSettings) -> io::Result<()> {
        let option_list = self.client.model_option_list()?;
        let option = option_list
            .iter()
            .find(|option| option.model == settings.model)
            .ok_or_else(|| io::Error::other("Choose an available model."))?;
        if settings
            .reasoning_effort
            .as_ref()
            .is_some_and(|effort| !option.reasoning_effort_list.contains(effort))
        {
            return Err(io::Error::other(
                "This reasoning level is not available for that model.",
            ));
        }
        Ok(())
    }

    fn validate_targets(&self, target_list: &[String]) -> io::Result<()> {
        let available = self.devices.target_list();
        if target_list
            .iter()
            .any(|id| !available.iter().any(|target| &target.id == id))
        {
            return Err(io::Error::other(
                "Register the execution device before granting conversation access.",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_keeps_newer_conversation_first_after_restart() {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let store = ServiceStore::load(directory.clone()).unwrap();
        let first = "10000000-0000-4000-8000-000000000000";
        let second = "90000000-0000-4000-8000-000000000000";
        let receipt = "a0000000-0000-4000-8000-000000000000";
        let sender = Target {
            id: "mac".into(),
            name: "Mac".into(),
        };

        store.create(&sender.id, first, "First").unwrap();
        store.record_user_message(first, "Saved question").unwrap();
        store.create(&sender.id, second, "Second").unwrap();
        store.record_user_message(second, "Newer question").unwrap();
        let first_sequence = store.get(first).unwrap().activity_sequence;

        // The service persisted the message before the client lost its acknowledgment.
        let path = store
            .conversation_dir(first)
            .unwrap()
            .join("conversation.json");
        let transcript = json!({
            "id":first, "model":"model", "status":"Ready", "input_list":[],
            "entry_list":[{"role":"You","text":"Saved question","message_id":receipt,"sender":sender}]
        });
        std::fs::write(path, serde_json::to_vec(&transcript).unwrap()).unwrap();
        drop(store);

        let store = Arc::new(ServiceStore::load(directory.clone()).unwrap());
        let client = Arc::new(ChatGptClient::new_central(&directory).unwrap());
        let devices = ToolConnection::new(crate::devices::device_with_kind("service", false));
        let service = CentralService::new(store.clone(), client, devices);
        let send = json!({"action":"send", "id":first, "message_id":receipt, "model":"model", "prompt":"Saved question"});
        assert_eq!(
            service.handle(sender.clone(), send).unwrap(),
            json!({"accepted":true})
        );
        assert_eq!(store.get(first).unwrap().activity_sequence, first_sequence);

        let listing = service.handle(sender, json!({"action":"list"})).unwrap();
        let ids: Vec<_> = listing["conversation_list"]
            .as_array()
            .unwrap()
            .iter()
            .map(|record| record["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids, [second, first]);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
