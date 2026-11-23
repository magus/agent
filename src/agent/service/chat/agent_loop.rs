use super::conversation::Conversation;
use super::model_request::model_request;
use super::response_preview::ResponsePreview;
use crate::devices::ToolConnection;
use crate::protocol::Target;
use crate::protocol::{ConversationView, Entry, ToolRequest};
use crate::service::ServiceStore;
use crate::service::chatgpt::ChatGptClient;
use base64::Engine;
use serde_json::{Value, json};
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

pub(crate) struct AgentLoop {
    pub client: Arc<ChatGptClient>,
    pub devices: Arc<ToolConnection>,
    pub(super) conversation: Mutex<Conversation>,
    pub(super) directory: PathBuf,
    pub(super) running: AtomicBool,
    pub(super) stopped: AtomicBool,
    submission: Mutex<()>,
    pub(super) execution: Mutex<()>,
    policy: Arc<ServiceStore>,
    id: String,
    pub(super) local: Target,
    pub(super) revision: AtomicU64,
    pub(super) run_id: AtomicU64,
    response_preview: Mutex<ResponsePreview>,
}

impl AgentLoop {
    pub fn load(
        client: Arc<ChatGptClient>,
        devices: Arc<ToolConnection>,
        directory: PathBuf,
        policy: Arc<ServiceStore>,
        id: String,
    ) -> io::Result<Arc<Self>> {
        let is_new = !directory.join("conversation.json").exists();
        let mut conversation = Conversation::load(&directory)?;
        if is_new {
            let settings = policy.get(&id)?.model_settings;
            conversation.model = settings.model;
            conversation.reasoning_effort = settings.reasoning_effort;
        }
        let local = devices.local_device();
        conversation.id = id.clone();
        conversation.save(&directory)?;
        let agent = Arc::new(Self {
            client,
            devices,
            local,
            policy,
            id,
            execution: Mutex::new(()),
            conversation: Mutex::new(conversation),
            directory,
            running: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            submission: Mutex::new(()),
            revision: AtomicU64::new(0),
            run_id: AtomicU64::new(0),
            response_preview: Mutex::new(ResponsePreview::default()),
        });
        Ok(agent)
    }

    pub fn allowed_target_list(&self) -> io::Result<Vec<Target>> {
        let record = self.policy.get(&self.id)?;
        Ok(self
            .devices
            .target_list()
            .into_iter()
            .filter(|target| record.allowed_target_list.contains(&target.id))
            .collect())
    }

    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Relaxed)
    }

    pub fn view(&self) -> ConversationView {
        let conversation = self.conversation.lock().unwrap();
        let mut entry_list = conversation.entry_list.clone();
        self.response_preview
            .lock()
            .unwrap()
            .append_to(&mut entry_list);
        ConversationView {
            id: conversation.id.clone(),
            model: conversation.model.clone(),
            reasoning_effort: conversation.reasoning_effort.clone(),
            status: conversation.status.clone(),
            entry_list,
        }
    }

    pub fn running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_model_settings(&self, settings: crate::protocol::ModelSettings) -> io::Result<()> {
        let _execution = self.execution.lock().unwrap();
        self.change(|conversation| {
            conversation.model = settings.model;
            conversation.reasoning_effort = settings.reasoning_effort;
            Ok(())
        })
    }

    pub fn start(
        self: &Arc<Self>,
        sender: Target,
        message_id: String,
        prompt: String,
        settings: crate::protocol::ModelSettings,
    ) -> io::Result<()> {
        uuid::Uuid::parse_str(&message_id).map_err(io::Error::other)?;
        let _execution = self.execution.lock().unwrap();
        if self
            .conversation
            .lock()
            .unwrap()
            .has_message(&message_id, &sender, &prompt)?
        {
            return Ok(());
        }
        if prompt.trim().is_empty() || settings.model.is_empty() {
            return Err(io::Error::other("Choose a model, then enter a prompt."));
        }
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(io::Error::other("A task is already running."));
        }
        let result = self.change(|conversation| {
            if conversation.pending() {
                return Err(io::Error::other(
                    "Retrieve pending tool results before sending another prompt.",
                ));
            }
            conversation.model = settings.model.clone();
            conversation.reasoning_effort = settings.reasoning_effort.clone();
            conversation.status = "Running".into();
            let mut entry = Entry::text("You", &prompt);
            entry.sender = Some(sender.clone());
            entry.message_id = Some(message_id);
            conversation.entry_list.push(entry);
            let content = format!(
                "Message sent from device {} ({})\n\n{}",
                sender.name, sender.id, prompt
            );
            conversation
                .input_list
                .push(json!({"role":"user", "content":content}));
            Ok(())
        });
        if let Err(error) = result {
            self.running.store(false, Ordering::SeqCst);
            return Err(error);
        }
        self.run_id.fetch_add(1, Ordering::SeqCst);
        self.stopped.store(false, Ordering::SeqCst);
        let agent = self.clone();
        std::thread::spawn(move || {
            if let Err(error) = agent.run(settings) {
                agent.report(error.to_string());
            }
            let _ = agent.change(|conversation| {
                conversation.finish_unsubmitted_calls();
                Ok(())
            });
            agent.running.store(false, Ordering::SeqCst);
        });
        Ok(())
    }

    pub fn stop(self: &Arc<Self>) {
        self.stopped.store(true, Ordering::SeqCst);
        let run_id = self.run_id.load(Ordering::SeqCst);
        let request_list: Vec<ToolRequest> = self
            .conversation
            .lock()
            .unwrap()
            .entry_list
            .iter()
            .filter_map(|entry| entry.request.clone())
            .filter(|request| !request.finished)
            .collect();
        let agent = self.clone();
        std::thread::spawn(move || {
            let _submission = agent.submission.lock().unwrap();
            for request in request_list {
                let message = match agent.devices.cancel(&request.host, &request.id) {
                    Ok(_) => "Stop requested. Submitted actions may already have acted; retrieve their results.".into(),
                    Err(error) => format!("Stopped. Cancellation could not be confirmed. {error}"),
                };
                if agent.run_id.load(Ordering::SeqCst) == run_id {
                    agent.report(message);
                }
            }
        });
    }

    pub fn retrieve(self: &Arc<Self>) -> io::Result<()> {
        let _execution = self.execution.lock().unwrap();
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(io::Error::other("Wait for the current task to stop."));
        }
        let agent = self.clone();
        std::thread::spawn(move || {
            let request_list: Vec<ToolRequest> = agent
                .conversation
                .lock()
                .unwrap()
                .entry_list
                .iter()
                .filter_map(|entry| entry.request.clone())
                .filter(|request| !request.finished)
                .collect();
            let result = (|| {
                for request in request_list {
                    let job = match agent.policy.with_target_access(
                        &agent.id,
                        &request.host,
                        || {
                            agent
                                .devices
                                .status_for(&request.host, &request.id, &request.owner)
                        },
                    ) {
                        Ok(job) => job,
                        Err(error) if error.to_string() == "Request not found" => {
                            json!({"status":"not_found", "result":{"error":"No saved result is accessible to this device. No action was repeated."}})
                        }
                        Err(error) => return Err(error),
                    };
                    agent.record_result(&request.id, job)?;
                }
                agent.change(|conversation| {
                    conversation.status = if conversation.pending() {
                        "A tool is still running. Retrieve again to check."
                    } else {
                        "Results retrieved. Send a prompt to continue."
                    }
                    .into();
                    Ok(())
                })
            })();
            if let Err(error) = result {
                agent.report(format!(
                    "Could not retrieve results. No action was repeated. {error}"
                ));
            }
            agent.running.store(false, Ordering::SeqCst);
        });
        Ok(())
    }

    pub(super) fn run(&self, settings: crate::protocol::ModelSettings) -> io::Result<()> {
        let _preview_cleanup = ResponsePreviewCleanup(self);
        let mut next_search = None;
        let mut image_reply = None;
        loop {
            self.check_stopped()?;
            let mut target_list = Vec::new();
            for target in self.allowed_target_list()? {
                let info = self.devices.info(&target.id);
                target_list.push(json!({"host":target.id, "name":target.name,
                    "tools":info.as_ref().ok().map(|info| &info["tool_list"]),
                    "unavailable_tools":info.as_ref().ok().map(|info| &info["unavailable_tools"]),
                    "error":info.err().map(|error| error.to_string())}));
            }
            let mut request = {
                let conversation = self.conversation.lock().unwrap();
                model_request(
                    &settings,
                    &conversation.input_list,
                    &target_list,
                    &self.local,
                )
            };
            let operation = if let Some(tool) = next_search.take() {
                super::search::enable(&mut request, tool);
                "search"
            } else {
                "chat"
            };
            let image_search = request["include"]
                .as_array()
                .is_some_and(|list| list.iter().any(|field| field == "web_search_call.results"));
            if let Some(results) = image_reply.take() {
                request["input"].as_array_mut().unwrap().push(json!({
                    "role": "developer",
                    "content": "Answer the user's request using the image results below as source data. Choose any pictures to display yourself. For an inline picture, copy its exact image_url into Markdown image syntax."
                }));
                request["input"].as_array_mut().unwrap().push(json!({
                    "role": "user", "content": format!("Image search results (JSON): {results}")
                }));
                request["tool_choice"] = json!("none");
            }
            let mut response = self.request_response(request, operation, !image_search)?;
            self.check_stopped()?;
            let output_list = response["output"]
                .as_array_mut()
                .ok_or_else(|| io::Error::other("Model returned no output."))?;
            for item in output_list.iter_mut() {
                super::search::strip_sources(item);
            }
            if image_search
                && !output_list
                    .iter()
                    .any(|item| item["type"] == "function_call")
            {
                let images: Vec<_> = output_list
                    .iter()
                    .filter(|item| item["type"] == "web_search_call")
                    .filter_map(|item| item["results"].as_array())
                    .flatten()
                    .filter(|item| item["type"] == "image_result" && item["image_url"].is_string())
                    .map(|item| {
                        json!({"image_url": item["image_url"], "caption": item["caption"],
                                       "source_website_url": item["source_website_url"]})
                    })
                    .collect();
                if !images.is_empty() {
                    image_reply = Some(json!(images));
                    output_list.retain(|item| item["type"] != "message");
                }
            }
            self.change(|conversation| {
                conversation.input_list.extend(output_list.iter().cloned());
                for output in output_list.iter() {
                    if let Some(query) = super::search::query_for_call(output) {
                        conversation.entry_list.push(Entry::text("Search", query));
                    } else if output["type"] == "web_search_call" {
                        if let Some(entry) = conversation
                            .entry_list
                            .iter_mut()
                            .rev()
                            .find(|entry| entry.role == "Search")
                        {
                            super::search::append_images(entry, output);
                        }
                    } else if output["type"] == "message" {
                        let text = output["content"]
                            .as_array()
                            .into_iter()
                            .flatten()
                            .filter_map(super::cited_text::cited_text)
                            .collect::<Vec<_>>()
                            .join("\n");
                        if !text.is_empty() {
                            conversation.entry_list.push(Entry::text("Agent", text));
                        }
                    }
                }
                self.response_preview.lock().unwrap().clear();
                Ok(())
            })?;
            let call_list: Vec<&Value> = output_list
                .iter()
                .filter(|output| output["type"] == "function_call")
                .collect();
            if image_reply.is_some() {
                continue;
            }
            if call_list.is_empty() {
                return self.change(|conversation| {
                    conversation.status = "Ready".into();
                    Ok(())
                });
            }
            for call in call_list {
                if call["name"] == "search" {
                    next_search = self.prepare_search(call)?;
                } else {
                    self.execute_call(call)?;
                }
            }
        }
    }

    fn request_response(
        &self,
        request: Value,
        operation: &'static str,
        preview: bool,
    ) -> io::Result<Value> {
        self.check_stopped()?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let mut journal = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.directory.join("openai-events.jsonl"))?;
        let client = self.client.clone();
        let (sender, receiver) = mpsc::sync_channel(32);
        std::thread::spawn(move || {
            let result = client.respond_with_text(
                &request,
                |index, text| {
                    if !preview {
                        return Ok(());
                    }
                    sender
                        .send(ModelEvent::Text {
                            index,
                            text: text.into(),
                        })
                        .map_err(|_| io::Error::new(io::ErrorKind::Interrupted, "Stopped"))
                },
                |event| {
                    use std::io::Write;
                    let mut record = serde_json::to_vec(
                        &json!({"request_id":request_id,"operation":operation,"event":event}),
                    )?;
                    record.push(b'\n');
                    journal.write_all(&record)
                },
            );
            let _ = sender.send(ModelEvent::Completed(result));
        });
        loop {
            self.check_stopped()?;
            match receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(ModelEvent::Completed(result)) => return result,
                Ok(ModelEvent::Text { index, text }) => {
                    self.response_preview.lock().unwrap().append(index, &text);
                    self.revision.fetch_add(1, Ordering::Relaxed);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => {
                    return Err(io::Error::other("Model request ended without a response."));
                }
            }
        }
    }

    fn prepare_search(&self, call: &Value) -> io::Result<Option<Value>> {
        let call_id = call["call_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("Missing search call ID"))?;
        let arguments = call["arguments"].as_str().unwrap_or("");
        let (tool, output) = match super::search::hosted_tool(arguments) {
            Ok(tool) => (
                Some(tool),
                "Web search is configured for this conversation.".to_owned(),
            ),
            Err(error) => (None, error.to_string()),
        };
        self.change(|conversation| {
            conversation
                .input_list
                .push(json!({"type":"function_call_output","call_id":call_id,"output":output}));
            Ok(())
        })?;
        Ok(tool)
    }

    fn execute_call(&self, call: &Value) -> io::Result<()> {
        self.check_stopped()?;
        let call_id = call["call_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("Missing tool call ID"))?;
        let parsed = parse_call(call).and_then(|(host, tool)| {
            self.policy
                .with_target_access(&self.id, &host, || self.devices.validate(&host, &tool))?;
            Ok((host, tool))
        });
        let (host, tool) = match parsed {
            Ok(parsed) => parsed,
            Err(error) => {
                return self.change(|conversation| {
                    conversation.input_list.push(json!({"type":"function_call_output", "call_id":call_id, "output":error.to_string()}));
                    conversation.entry_list.push(Entry::text("Tool error", error.to_string()));
                    Ok(())
                });
            }
        };
        let id = uuid::Uuid::new_v4().to_string();
        let request = ToolRequest {
            owner: self.local.id.clone(),
            id: id.clone(),
            call_id: call_id.into(),
            host: host.clone(),
            tool: tool.clone(),
            status: "Submitting".into(),
            finished: false,
        };
        self.change(|conversation| {
            conversation.entry_list.push(Entry {
                sender: None,
                message_id: None,
                role: "Tool".into(),
                text: serde_json::to_string_pretty(&tool)?,
                image: None,
                request: Some(request),
                image_result_list: Vec::new(),
            });
            Ok(())
        })?;
        let result = {
            let _submission = self.submission.lock().unwrap();
            self.check_stopped()?;
            self.policy
                .with_target_access(&self.id, &host, || self.devices.submit(&host, &id, tool))
        };
        let mut job = match result {
            Ok(job) => job,
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                json!({"status":"denied", "result":{"error":error.to_string(), "dispatched":false}})
            }
            Err(error) => {
                return Err(io::Error::other(format!(
                    "Connection failed during submission. Retrieve request {id} before continuing. {error}"
                )));
            }
        };
        loop {
            let finished = job["status"] != "running";
            self.record_result(&id, job)?;
            if finished {
                break;
            }
            self.check_stopped()?;
            std::thread::sleep(Duration::from_millis(250));
            self.check_stopped()?;
            job = self.policy.with_target_access(&self.id, &host, || self.devices.status(&host, &id)).map_err(|error| {
                io::Error::other(format!(
                    "Connection interrupted. Retrieve request {id}; do not submit it again. {error}"
                ))
            })?;
        }
        Ok(())
    }

    fn record_result(&self, id: &str, mut job: Value) -> io::Result<()> {
        let finished = job["status"] != "running";
        let image_base64 = job["result"]
            .as_object_mut()
            .and_then(|result| result.remove("image_base64"));
        let image = if let Some(encoded) = image_base64.as_ref().and_then(Value::as_str) {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(io::Error::other)?;
            let path = self.directory.join(format!("{id}.jpg"));
            std::fs::write(&path, bytes)?;
            Some(path)
        } else {
            None
        };
        self.change(|conversation| {
            let entry = conversation.entry_list.iter_mut().find(|entry| entry.request.as_ref().is_some_and(|request| request.id == id))
                .ok_or_else(|| io::Error::other("Saved tool request is missing"))?;
            let request = entry.request.as_mut().unwrap();
            if request.finished { return Ok(()); }
            request.status = job["status"].as_str().unwrap_or("unknown").into();
            if !finished { return Ok(()); }
            request.finished = true;
            entry.image = image;
            entry.text = if request.tool["tool"] == "computer" {
                serde_json::to_string_pretty(&job)?
            } else {
                tool_summary(&request.tool, &job["result"])?
            };
            let text = serde_json::to_string(&job)?;
            conversation.input_list.push(json!({"type":"function_call_output", "call_id":request.call_id, "output":text}));
            if let Some(encoded) = image_base64.and_then(|value| value.as_str().map(str::to_owned)) {
                conversation.input_list.push(json!({"role":"user", "content":[
                    {"type":"input_text", "text":format!("Screenshot returned by tool request {id}. Inspect it before choosing another action.")},
                    {"type":"input_image", "image_url":format!("data:image/jpeg;base64,{encoded}"), "detail":"high"}]}));
            }
            Ok(())
        })
    }

    fn check_stopped(&self) -> io::Result<()> {
        if self.stopped.load(Ordering::SeqCst) {
            return Err(io::Error::other(
                "Stopped. No further model turns or tools will be submitted. Submitted actions cannot be undone.",
            ));
        }
        Ok(())
    }

    pub(super) fn change(
        &self,
        change: impl FnOnce(&mut Conversation) -> io::Result<()>,
    ) -> io::Result<()> {
        let mut conversation = self.conversation.lock().unwrap();
        let mut updated = conversation.clone();
        change(&mut updated)?;
        updated.save(&self.directory)?;
        *conversation = updated;
        self.revision.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    pub(super) fn report(&self, message: String) {
        if let Err(error) = self.change(|conversation| {
            conversation.status = message;
            Ok(())
        }) {
            eprintln!("Could not save conversation. {error}");
        }
    }
}

enum ModelEvent {
    Text { index: usize, text: String },
    Completed(io::Result<Value>),
}

struct ResponsePreviewCleanup<'a>(&'a AgentLoop);

impl Drop for ResponsePreviewCleanup<'_> {
    fn drop(&mut self) {
        self.0.response_preview.lock().unwrap().clear();
        self.0.revision.fetch_add(1, Ordering::Relaxed);
    }
}

fn parse_call(call: &Value) -> io::Result<(String, Value)> {
    let arguments: Value = serde_json::from_str(call["arguments"].as_str().unwrap_or(""))?;
    let host = arguments["host"]
        .as_str()
        .filter(|host| !host.is_empty())
        .ok_or_else(|| io::Error::other("Every tool call requires a host."))?
        .to_owned();
    let tool = match call["name"].as_str() {
        Some("shell") => {
            json!({"tool":"shell", "command":arguments["command"], "cwd":arguments["cwd"], "timeout_seconds":arguments["timeout_seconds"]})
        }
        Some("computer") => {
            let action: Value =
                serde_json::from_str(arguments["action_json"].as_str().unwrap_or(""))?;
            let operation = action["operation"].as_str().unwrap_or("");
            if !["capture", "click", "keys", "paste", "drag"].contains(&operation) {
                return Err(io::Error::other("Unsupported computer action"));
            }
            json!({"tool":"computer", "action":action})
        }
        _ => {
            return Err(io::Error::other(
                "Unknown tool. Only shell and computer are available.",
            ));
        }
    };
    Ok((host, tool))
}

fn tool_summary(tool: &Value, result: &Value) -> io::Result<String> {
    if tool["tool"] == "shell" {
        let exit_status = match result["exit_code"].as_i64() {
            Some(code) => format!("Exit {code}"),
            None => "Process ended without an exit code".into(),
        };
        return Ok(format!(
            "{}\n\n{}\n{}{}{}",
            tool["command"].as_str().unwrap_or(""),
            exit_status,
            result["stdout"].as_str().unwrap_or(""),
            result["stderr"].as_str().unwrap_or(""),
            result["error"].as_str().unwrap_or("")
        ));
    }
    let action = serde_json::to_string(&tool["action"])?;
    if result["ok"] == false || result.get("error").is_some() {
        return Ok(format!("{action}\n{}", serde_json::to_string(result)?));
    }
    Ok(action)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_calls_require_a_host_and_cannot_request_pairing() {
        assert!(parse_call(&json!({"name":"shell", "arguments":"{}"})).is_err());
        let arguments =
            json!({"host":"mini", "action_json":"{\"operation\":\"approve\"}"}).to_string();
        assert!(parse_call(&json!({"name":"computer", "arguments":arguments})).is_err());
        assert!(parse_call(&json!({"name":"approve", "arguments":"{\"host\":\"mini\"}"})).is_err());
        let arguments =
            json!({"host":"mini", "action_json":"{\"operation\":\"capture\"}"}).to_string();
        let (host, tool) = parse_call(&json!({"name":"computer", "arguments":arguments})).unwrap();
        assert_eq!(host, "mini");
        assert_eq!(tool["action"]["operation"], "capture");
    }
}
