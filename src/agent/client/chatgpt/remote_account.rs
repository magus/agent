use super::callback_server::CallbackServer;
use super::login_timing::LoginTiming;
use crate::client::chat::ChatClient;
use serde_json::json;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub(crate) struct RemoteAccount {
    chat: Arc<ChatClient>,
    callback_server: Mutex<Option<CallbackServer>>,
}

impl RemoteAccount {
    pub fn new(chat: Arc<ChatClient>) -> Self {
        Self {
            chat,
            callback_server: Mutex::new(None),
        }
    }

    pub(crate) fn sign_in_at(
        &self,
        host: &str,
        cancelled: &Arc<AtomicBool>,
        timing: &LoginTiming,
        open: impl FnOnce(&str) -> io::Result<()>,
    ) -> io::Result<String> {
        let registration = self.chat.request_at(
            host,
            json!({"action":"login", "request":{"action":"registration"}}),
        )?;
        let client_id = registration["client_id"]
            .as_str()
            .ok_or_else(|| io::Error::other("Missing login registration"))?;
        let attempt = {
            let mut server = self.callback_server.lock().unwrap();
            if server.is_none() {
                *server = Some(CallbackServer::start()?);
            }
            server.as_ref().unwrap().begin(client_id, cancelled.clone())
        };
        let begun = self.chat.request_at(
            host,
            json!({"action":"login", "request":{
                "action":"begin", "state":attempt.state, "redirect_uri":attempt.redirect_uri
            }}),
        )?;
        let url = begun["url"]
            .as_str()
            .ok_or_else(|| io::Error::other("Missing authorization URL"))?;
        let parsed = url::Url::parse(url).map_err(io::Error::other)?;
        if parsed.scheme() != "https" || parsed.host_str() != Some("auth.openai.com") {
            return Err(io::Error::other("Unexpected login destination."));
        }
        if let Err(error) = open(url) {
            cancelled.store(true, Ordering::SeqCst);
            let _ = self.chat.request_at(
                host,
                json!({"action":"login", "request":{"action":"cancel", "state":attempt.state}}),
            );
            return Err(error);
        }

        let finished = Arc::new(AtomicBool::new(false));
        let observer_finished = finished.clone();
        let observer_cancelled = cancelled.clone();
        let observer_chat = self.chat.clone();
        let observer_host = host.to_owned();
        let observer_state = attempt.state.clone();
        let observer = std::thread::spawn(move || {
            while !observer_finished.load(Ordering::SeqCst) {
                if observer_cancelled.load(Ordering::SeqCst) {
                    let _ = observer_chat.request_at(&observer_host, json!({"action":"login", "request":{"action":"cancel", "state":observer_state}}));
                    break;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        });
        let result = (|| {
            let (callback, mut stream) = attempt.receive()?;
            timing.mark("callback_received");
            let request = json!({"action":"login", "request":{"action":"complete", "state":attempt.state,
                "code":callback.code, "client_id":callback.client_id}});
            let mut result = self.chat.request_at(host, request.clone());
            // Retry the same attempt after a transport timeout, never exchange the code twice.
            for _ in 0..2 {
                let timed_out = result.as_ref().err().is_some_and(|error| {
                    matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    )
                });
                if !timed_out {
                    break;
                }
                result = self.chat.request_at(host, request.clone());
            }
            attempt.finish(&mut stream, result.is_ok());
            timing.mark("browser_reply_finished");
            let response = result?;
            Ok(response["account"].as_str().unwrap_or_default().into())
        })();
        if cancelled.load(Ordering::SeqCst) {
            let _ = self.chat.request_at(
                host,
                json!({"action":"login", "request":{
                    "action":"cancel", "state":attempt.state
                }}),
            );
        }
        finished.store(true, Ordering::SeqCst);
        let _ = observer.join();
        result
    }
}
