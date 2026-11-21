use super::message_delivery::{Delivery, DeliveryRequest, MessageDelivery};
use crate::devices::ToolConnection;
use crate::protocol::{ConversationRecord, ConversationResponse, ConversationView, ServiceListing};
use crate::protocol::{Entry, Target};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct ChatClient {
    devices: Arc<ToolConnection>,
    directory: PathBuf,
    selection: Mutex<Selection>,
    cached: Mutex<Cached>,
    revision: AtomicU64,
    sending: Mutex<()>,
    delivery: MessageDelivery,
    service_list: Mutex<Vec<Target>>,
}

impl ChatClient {
    pub fn load(devices: Arc<ToolConnection>, directory: PathBuf) -> io::Result<Arc<Self>> {
        fs::create_dir_all(&directory)?;
        let selection = match fs::read(directory.join("selection.json")) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Selection::default(),
            Err(error) => return Err(error),
        };
        let delivery = MessageDelivery::load(directory.join("pending_message.json"))?;
        let client = Arc::new(Self {
            delivery,
            devices,
            directory,
            selection: Mutex::new(selection),
            cached: Mutex::new(Cached::default()),
            revision: AtomicU64::new(0),
            sending: Mutex::new(()),
            service_list: Mutex::new(Vec::new()),
        });
        let weak = Arc::downgrade(&client);
        std::thread::spawn(move || {
            while let Some(client) = weak.upgrade() {
                if let Err(error) = client.refresh() {
                    client.cached.lock().unwrap().conversation.status =
                        format!("Disconnected. {error}");
                    client.revision.fetch_add(1, Ordering::Relaxed);
                }
                drop(client);
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        let weak = Arc::downgrade(&client);
        std::thread::spawn(move || {
            while let Some(client) = weak.upgrade() {
                client.discover_services();
                drop(client);
                std::thread::sleep(Duration::from_secs(15));
            }
        });
        Ok(client)
    }

    pub fn service_list(&self) -> Vec<Target> {
        self.service_list.lock().unwrap().clone()
    }

    pub fn connect(&self, host: String) -> io::Result<()> {
        self.devices
            .service_request(&host, json!({"action":"list"}))?;
        let mut selection = self.selection.lock().unwrap();
        *selection = Selection {
            host,
            conversation: String::new(),
        };
        self.save_selection(&selection)?;
        drop(selection);
        self.refresh()
    }

    pub fn service_name(&self) -> String {
        let host = self.selection.lock().unwrap().host.clone();
        self.service_list
            .lock()
            .unwrap()
            .iter()
            .find(|service| service.id == host)
            .map(|service| service.name.clone())
            .unwrap_or_else(|| "Choose service".into())
    }

    pub fn conversation_list(&self) -> Vec<ConversationRecord> {
        self.cached.lock().unwrap().conversation_list.clone()
    }

    pub fn select(&self, id: String) -> io::Result<()> {
        let mut selection = self.selection.lock().unwrap();
        selection.conversation = id;
        self.save_selection(&selection)?;
        drop(selection);
        self.refresh()
    }

    pub fn create(&self) -> io::Result<()> {
        let _sending = self.sending.lock().unwrap();
        let host = self.service_host()?;
        let path = self.directory.join("pending_create.json");
        let pending = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Value>(&bytes)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                json!({"host":host, "id":uuid::Uuid::new_v4().to_string()})
            }
            Err(error) => return Err(error),
        };
        if pending["host"] != host {
            return Err(io::Error::other(
                "Reconnect to the previous service to finish creating its conversation.",
            ));
        }
        save_json(&path, &pending)?;
        let id = pending["id"]
            .as_str()
            .ok_or_else(|| io::Error::other("Invalid pending conversation"))?;
        uuid::Uuid::parse_str(id).map_err(io::Error::other)?;
        let title = format!("Conversation {}", &id[..8]);
        self.request_at(&host, json!({"action":"create", "id":id, "title":title}))?;
        self.select(id.into())?;
        fs::remove_file(path)
    }

    pub fn view(&self) -> ConversationView {
        self.cached.lock().unwrap().conversation.clone()
    }
    pub fn running(&self) -> bool {
        self.cached.lock().unwrap().running
    }
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }
    pub fn target_list(&self) -> Vec<Target> {
        self.cached.lock().unwrap().target_list.clone()
    }
    pub fn available_target_list(&self) -> Vec<Target> {
        self.cached.lock().unwrap().available_target_list.clone()
    }
    pub fn default_target_list(&self) -> Vec<String> {
        self.cached.lock().unwrap().default_target_list.clone()
    }

    pub fn model_settings(&self) -> crate::protocol::ModelSettings {
        self.cached.lock().unwrap().model_settings.clone()
    }

    pub fn model_option_list(&self) -> io::Result<Vec<crate::protocol::ModelOption>> {
        let response = self.request(json!({"action":"model_options"}))?;
        serde_json::from_value(response["model_option_list"].clone()).map_err(io::Error::other)
    }

    pub fn set_model_settings(&self, settings: crate::protocol::ModelSettings) -> io::Result<()> {
        self.request(json!({"action":"set_model_defaults", "settings":settings}))?;
        self.refresh()
    }

    pub fn set_conversation_model(
        &self,
        service: &str,
        id: &str,
        settings: crate::protocol::ModelSettings,
    ) -> io::Result<()> {
        self.request_at(
            service,
            json!({"action":"set_conversation_model", "id":id, "settings":settings}),
        )?;
        self.refresh()
    }

    pub fn set_targets(&self, target_list: Vec<String>, defaults: bool) -> io::Result<()> {
        let action = if defaults {
            "set_defaults"
        } else {
            "set_targets"
        };
        let mut request = json!({"action":action, "target_list":target_list});
        if !defaults {
            request["id"] = self.selection.lock().unwrap().conversation.clone().into();
        }
        self.request(request)?;
        self.refresh()
    }

    pub fn set_conversation_targets(
        &self,
        service: &str,
        id: &str,
        target_list: Vec<String>,
    ) -> io::Result<()> {
        self.request_at(
            service,
            json!({"action":"set_targets", "id":id, "target_list":target_list}),
        )?;
        self.refresh()
    }

    pub fn start(
        &self,
        service_host: String,
        conversation_id: String,
        message_id: String,
        prompt: String,
        settings: crate::protocol::ModelSettings,
    ) -> io::Result<String> {
        let _sending = self.sending.lock().unwrap();
        let id = &conversation_id;
        if id.is_empty() {
            return Err(io::Error::other("Create or open a conversation first."));
        }
        let delivery = self.delivery.begin(Delivery {
            service_host: service_host.clone(),
            request: DeliveryRequest {
                action: "send".into(),
                id: conversation_id.clone(),
                message_id,
                prompt,
                settings,
            },
            failure: None,
        })?;
        let result = (|| {
            let request = serde_json::to_value(&delivery.request)?;
            let response = self.request_at(&service_host, request)?;
            if response["accepted"] != true {
                return Err(io::Error::other(
                    response["error"]
                        .as_str()
                        .unwrap_or("Message was not accepted."),
                ));
            }
            self.delivery.accepted()
        })();
        if let Err(error) = result {
            self.delivery.failed(&error);
            return Err(error);
        }
        let message_id = delivery.request.message_id;
        let prompt = delivery.request.prompt;
        let settings = delivery.request.settings;
        let selection = self.selection.lock().unwrap();
        if selection.host == service_host && selection.conversation == conversation_id {
            let mut cached = self.cached.lock().unwrap();
            if cached.conversation.id == conversation_id {
                let already_visible = cached
                    .conversation
                    .entry_list
                    .iter()
                    .any(|entry| entry.message_id.as_deref() == Some(&message_id));
                if !already_visible {
                    let mut entry = Entry::text("You", prompt);
                    entry.message_id = Some(message_id.clone());
                    entry.sender = Some(self.devices.local_device());
                    cached.conversation.entry_list.push(entry);
                    cached.conversation.model = settings.model;
                    cached.conversation.reasoning_effort = settings.reasoning_effort;
                    cached.conversation.status = "Running".into();
                    cached.running = true;
                    self.revision.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        Ok(message_id)
    }

    pub fn prepare_delivery(&self, delivery: Delivery) -> io::Result<Value> {
        if delivery.service_host.is_empty() || delivery.request.id.is_empty() {
            return Err(io::Error::other("Create or open a conversation first."));
        }
        self.delivery.begin(delivery)?;
        Ok(self.pending_delivery())
    }

    pub fn pending_delivery(&self) -> Value {
        match self.delivery.pending() {
            Some(delivery) => json!({
                "service": delivery.service_host,
                "conversation_id": delivery.request.id,
                "message_id": delivery.request.message_id,
                "prompt": delivery.request.prompt,
                "settings": delivery.request.settings,
                "failure": delivery.failure,
            }),
            None => Value::Null,
        }
    }

    pub fn retry_delivery(&self) -> io::Result<String> {
        let delivery = self
            .delivery
            .pending()
            .ok_or_else(|| io::Error::other("No message is waiting for retry."))?;
        self.start(
            delivery.service_host,
            delivery.request.id,
            delivery.request.message_id,
            delivery.request.prompt,
            delivery.request.settings,
        )
    }

    pub fn stop(&self) -> io::Result<()> {
        let id = self.selection.lock().unwrap().conversation.clone();
        self.request(json!({"action":"stop", "id":id}))?;
        Ok(())
    }

    pub fn retrieve(&self) -> io::Result<()> {
        let id = self.selection.lock().unwrap().conversation.clone();
        self.request(json!({"action":"retrieve", "id":id}))?;
        Ok(())
    }

    fn discover_services(&self) {
        let mut service_list = Vec::new();
        for target in self.devices.target_list() {
            if target.id == self.devices.local_device().id {
                continue;
            }
            if let Ok(info) = self.devices.info(&target.id)
                && info["tool_list"]
                    .as_array()
                    .is_some_and(|list| list.iter().any(|tool| tool == "conversations"))
            {
                service_list.push(target);
            }
        }
        *self.service_list.lock().unwrap() = service_list;
    }

    fn refresh(&self) -> io::Result<()> {
        let revision = self.revision.load(Ordering::Relaxed);
        let selection = self.selection.lock().unwrap().clone();
        if selection.host.is_empty() {
            return Err(io::Error::other("Choose a paired central service."));
        }
        let listed = self
            .devices
            .service_request(&selection.host, json!({"action":"list"}))?;
        let listed: ServiceListing = serde_json::from_value(listed)?;
        let conversation_list = listed.conversation_list;
        if selection.conversation.is_empty()
            && let Some(id) = conversation_list.first().map(|record| record.id.as_str())
        {
            let mut current = self.selection.lock().unwrap();
            if current.host != selection.host || !current.conversation.is_empty() {
                return Ok(());
            }
            current.conversation = id.into();
            self.save_selection(&current)?;
        }
        let id = {
            let current = self.selection.lock().unwrap();
            if current.host != selection.host {
                return Ok(());
            }
            current.conversation.clone()
        };
        let mut cached = Cached {
            conversation_list,
            available_target_list: listed.target_list,
            default_target_list: listed.default_target_list,
            model_settings: listed.model_settings,
            ..Default::default()
        };
        if !id.is_empty() {
            let known_image_list: Vec<String> = fs::read_dir(&self.directory)?
                .flatten()
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name.ends_with(".jpg"))
                .collect();
            let result = self.devices.service_request(
                &selection.host,
                json!({"action":"get", "id":id, "known_image_list":known_image_list}),
            )?;
            let result: ConversationResponse = serde_json::from_value(result)?;
            cached.conversation = result.conversation;
            cached.running = result.running;
            cached.target_list = result.target_list;
            for entry in &mut cached.conversation.entry_list {
                if let Some(name) = &entry.image {
                    let name = name
                        .to_str()
                        .ok_or_else(|| io::Error::other("Invalid image name"))?;
                    let image_id = name
                        .strip_suffix(".jpg")
                        .ok_or_else(|| io::Error::other("Invalid image extension"))?;
                    uuid::Uuid::parse_str(image_id).map_err(io::Error::other)?;
                    let path = self.directory.join(name);
                    if !path.exists() {
                        let encoded = result
                            .image_by_id
                            .get(name)
                            .ok_or_else(|| io::Error::other("Missing screenshot"))?;
                        fs::write(
                            &path,
                            base64::engine::general_purpose::STANDARD
                                .decode(encoded)
                                .map_err(io::Error::other)?,
                        )?;
                    }
                    entry.image = Some(path);
                }
            }
        } else {
            cached.conversation.status = "Connected. Create a conversation.".into();
        }
        let current = self.selection.lock().unwrap();
        if current.host == selection.host
            && current.conversation == id
            && self.revision.load(Ordering::Relaxed) == revision
        {
            *self.cached.lock().unwrap() = cached;
            self.revision.fetch_add(1, Ordering::Relaxed);
        }
        Ok(())
    }

    pub(crate) fn service_host(&self) -> io::Result<String> {
        let host = self.selection.lock().unwrap().host.clone();
        if host.is_empty() {
            return Err(io::Error::other("Choose a central service first."));
        }
        Ok(host)
    }

    pub(crate) fn request_at(&self, host: &str, request: Value) -> io::Result<Value> {
        self.devices.service_request(host, request)
    }

    pub(crate) fn request(&self, request: Value) -> io::Result<Value> {
        let host = self.selection.lock().unwrap().host.clone();
        if host.is_empty() {
            return Err(io::Error::other("Choose a central service first."));
        }
        self.devices.service_request(&host, request)
    }

    fn save_selection(&self, selection: &Selection) -> io::Result<()> {
        save_json(&self.directory.join("selection.json"), selection)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
struct Selection {
    host: String,
    conversation: String,
}

#[derive(Default)]
struct Cached {
    model_settings: crate::protocol::ModelSettings,
    conversation: ConversationView,
    running: bool,
    conversation_list: Vec<ConversationRecord>,
    target_list: Vec<Target>,
    available_target_list: Vec<Target>,
    default_target_list: Vec<String>,
}

pub(super) fn save_json(path: &std::path::Path, value: &impl Serialize) -> io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let temporary = path.with_extension("tmp");
    let mut file = fs::OpenOptions::new()
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
