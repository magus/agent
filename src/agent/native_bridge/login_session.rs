use crate::client::chat::ChatClient;
use crate::client::chatgpt::{LoginTiming, RemoteAccount};
use serde::Serialize;
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[cfg(test)]
mod tests;

pub(super) struct LoginSession {
    account: RemoteAccount,
    state: Mutex<LoginState>,
}

impl LoginSession {
    pub(super) fn new(chat: Arc<ChatClient>) -> Arc<Self> {
        Arc::new(Self {
            account: RemoteAccount::new(chat),
            state: Mutex::new(LoginState::default()),
        })
    }

    pub(super) fn start(self: &Arc<Self>, service: String, id: String) -> io::Result<()> {
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut state = self.state.lock().unwrap();
            if state.view.pending {
                return Err(io::Error::other("A login is already in progress."));
            }
            *state = LoginState {
                view: LoginView {
                    id,
                    service: service.clone(),
                    pending: true,
                    ..LoginView::default()
                },
                cancelled: cancelled.clone(),
            };
        }

        let session = self.clone();
        let worker_cancelled = cancelled.clone();
        let worker = std::thread::Builder::new()
            .name("native-login".into())
            .spawn(move || {
                let cancelled = worker_cancelled;
                let result =
                    session
                        .account
                        .sign_in_at(&service, &cancelled, &LoginTiming::new(), |url| {
                            session.state.lock().unwrap().publish_url(&cancelled, url)
                        });

                session.state.lock().unwrap().finish(&cancelled, result);
            });
        if let Err(error) = worker {
            let mut state = self.state.lock().unwrap();
            if Arc::ptr_eq(&state.cancelled, &cancelled) {
                state.view.pending = false;
            }
            return Err(error);
        }
        Ok(())
    }

    pub(super) fn cancel(&self) {
        self.state.lock().unwrap().cancel();
    }

    pub(super) fn view(&self) -> LoginView {
        self.state.lock().unwrap().view.clone()
    }
}

#[derive(Clone, Default, Serialize)]
pub(super) struct LoginView {
    id: String,
    service: String,
    pending: bool,
    url: Option<String>,
    account: Option<String>,
    error: Option<String>,
    completed: bool,
}

#[derive(Default)]
struct LoginState {
    view: LoginView,
    cancelled: Arc<AtomicBool>,
}

impl LoginState {
    fn cancel(&mut self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.view.pending = false;
        self.view.url = None;
    }

    fn publish_url(&mut self, cancelled: &Arc<AtomicBool>, url: &str) -> io::Result<()> {
        if !Arc::ptr_eq(&self.cancelled, cancelled) || cancelled.load(Ordering::SeqCst) {
            return Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "Login cancelled",
            ));
        }

        self.view.url = Some(url.into());
        Ok(())
    }

    fn finish(&mut self, cancelled: &Arc<AtomicBool>, result: io::Result<String>) {
        if !Arc::ptr_eq(&self.cancelled, cancelled) || cancelled.load(Ordering::SeqCst) {
            return;
        }

        self.view.pending = false;
        self.view.url = None;
        match result {
            Ok(account) => {
                self.view.account = Some(account);
                self.view.completed = true;
            }
            Err(error) => self.view.error = Some(error.to_string()),
        }
    }
}
