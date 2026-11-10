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
