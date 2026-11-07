use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;
use url::Url;

pub(super) struct CallbackServer {
    address: SocketAddr,
    attempt_by_state: Arc<Mutex<HashMap<String, AttemptState>>>,
    stopped: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl CallbackServer {
    pub(super) fn start() -> io::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let address = listener.local_addr()?;
        let attempt_by_state = Arc::new(Mutex::new(HashMap::new()));
        let stopped = Arc::new(AtomicBool::new(false));
        let worker_attempts = attempt_by_state.clone();
        let worker_stopped = stopped.clone();
        let redirect_uri = format!("http://{address}/auth/callback");

        let worker = std::thread::Builder::new()
            .name("login-callback".into())
            .spawn(move || {
                for stream in listener.incoming() {
                    if worker_stopped.load(Ordering::Relaxed) {
                        break;
                    }
                    let Ok(stream) = stream else { break };
                    let _ = route_callback(stream, &redirect_uri, &worker_attempts);
                }
            })?;

        Ok(Self {
            address,
            attempt_by_state,
            stopped,
            worker: Some(worker),
        })
    }

    pub(super) fn begin(&self, client_id: &str, cancelled: Arc<AtomicBool>) -> LoginAttempt {
        let state = uuid::Uuid::new_v4().simple().to_string();
        let (sender, receiver) = mpsc::channel();
        self.attempt_by_state.lock().unwrap().insert(
            state.clone(),
            AttemptState {
                client_id: client_id.into(),
                cancelled: cancelled.clone(),
                sender: Some(sender),
                result: LoginResult::Pending,
            },
        );

        LoginAttempt {
            state,
            redirect_uri: format!("http://{}/auth/callback", self.address),
            receiver,
            cancelled,
            attempt_by_state: self.attempt_by_state.clone(),
        }
    }
}

impl Drop for CallbackServer {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        // Wake the blocking accept only when the owner shuts down.
        let _ = TcpStream::connect(self.address);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

pub(super) struct LoginAttempt {
    pub(super) state: String,
    pub(super) redirect_uri: String,
    receiver: mpsc::Receiver<io::Result<(Callback, TcpStream)>>,
    cancelled: Arc<AtomicBool>,
    attempt_by_state: Arc<Mutex<HashMap<String, AttemptState>>>,
}

impl LoginAttempt {
    pub(super) fn receive(&self) -> io::Result<(Callback, TcpStream)> {
        loop {
            if self.cancelled.load(Ordering::Relaxed) {
                self.set_result(LoginResult::Cancelled);
                return Err(io::Error::other("Login cancelled."));
            }
            match self.receiver.recv_timeout(Duration::from_millis(100)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => continue,
                Err(_) => return Err(io::Error::other("Login callback server stopped.")),
            }
        }
    }

    pub(super) fn finish(&self, stream: &mut TcpStream, connected: bool) {
        let result = if connected {
            LoginResult::Connected
        } else if self.cancelled.load(Ordering::Relaxed) {
            LoginResult::Cancelled
        } else {
            LoginResult::Failed
        };
        self.set_result(result);
        // Closing the browser must not undo credentials already saved by the client.
        let _ = send_login_result(stream, result);
    }

    fn set_result(&self, result: LoginResult) {
        if let Some(attempt) = self.attempt_by_state.lock().unwrap().get_mut(&self.state) {
            attempt.result = result;
            attempt.sender = None;
        }
    }
}

impl Drop for LoginAttempt {
    fn drop(&mut self) {
        let mut attempt_by_state = self.attempt_by_state.lock().unwrap();
        if let Some(attempt) = attempt_by_state.get_mut(&self.state)
            && matches!(attempt.result, LoginResult::Pending)
        {
            attempt.result = LoginResult::Cancelled;
            attempt.sender = None;
        }
        drop(attempt_by_state);

        // A callback can arrive just as Cancel is clicked.
        while let Ok(Ok((_, mut stream))) = self.receiver.try_recv() {
            let _ = send_login_result(&mut stream, LoginResult::Cancelled);
        }
    }
}

pub(super) struct Callback {
    pub(super) code: String,
    pub(super) client_id: String,
}

struct AttemptState {
    client_id: String,
    cancelled: Arc<AtomicBool>,
    sender: Option<mpsc::Sender<io::Result<(Callback, TcpStream)>>>,
    result: LoginResult,
}

#[derive(Clone, Copy)]
enum LoginResult {
    Pending,
    Connected,
    Cancelled,
    Failed,
}

fn route_callback(
    mut stream: TcpStream,
    redirect_uri: &str,
    attempt_by_state: &Mutex<HashMap<String, AttemptState>>,
) -> io::Result<()> {
    stream.set_read_timeout(Some(Duration::from_secs(2)))?;
    stream.set_write_timeout(Some(Duration::from_secs(2)))?;
    let mut line = String::new();
    BufReader::new((&mut stream).take(16 * 1024)).read_line(&mut line)?;
    let mut part_list = line.split_whitespace();
    let method = part_list.next();
    let path = part_list.next().unwrap_or("");
    let base = Url::parse(redirect_uri).unwrap();
    let callback_url = base.join(path).map_err(io::Error::other)?;
    let state_list: Vec<_> = callback_url
        .query_pairs()
        .filter(|(key, _)| key == "state")
        .collect();
    if method != Some("GET")
        || callback_url.origin() != base.origin()
        || callback_url.path() != base.path()
        || state_list.len() != 1
    {
        return send_login_result(&mut stream, LoginResult::Failed);
    }

    let mut attempt_by_state = attempt_by_state.lock().unwrap();
    let Some(attempt) = attempt_by_state.get_mut(state_list[0].1.as_ref()) else {
        drop(attempt_by_state);
        return send_login_result(&mut stream, LoginResult::Failed);
    };
    if matches!(attempt.result, LoginResult::Pending) && attempt.cancelled.load(Ordering::Relaxed) {
        attempt.result = LoginResult::Cancelled;
        attempt.sender = None;
    }

    let Some(sender) = attempt.sender.as_ref() else {
        let result = attempt.result;
        drop(attempt_by_state);
        return send_login_result(&mut stream, result);
    };
    let callback = parse_callback(
        path,
        redirect_uri,
        state_list[0].1.as_ref(),
        &attempt.client_id,
    );
    match callback {
        Ok(callback) => {
            // Consume the sender before handing off the code, so retries cannot exchange it twice.
            let sender = attempt.sender.take().unwrap();
            if let Err(error) = sender.send(Ok((callback, stream))) {
                attempt.result = LoginResult::Cancelled;
                drop(attempt_by_state);
                if let Ok((_, mut stream)) = error.0 {
                    send_login_result(&mut stream, LoginResult::Cancelled)?;
                }
            }
        }
        Err(error) => {
            let denied = callback_url
                .query_pairs()
                .any(|(key, value)| key == "error" && value == "access_denied");
            let result = if denied {
                LoginResult::Cancelled
            } else {
                LoginResult::Failed
            };
            // Invalid callbacks do not terminate a valid pending attempt.
            if error.kind() == io::ErrorKind::PermissionDenied {
                let _ = sender.send(Err(error));
                attempt.sender = None;
                attempt.result = result;
            }
            drop(attempt_by_state);
            send_login_result(&mut stream, result)?;
        }
    }
    Ok(())
}

fn send_login_result(stream: &mut TcpStream, result: LoginResult) -> io::Result<()> {
    let (state, symbol, title, message) = match result {
        LoginResult::Connected => (
            "connected",
            "✓",
            "Agent connected",
            "Agent is connected to your ChatGPT account and ready to use.",
        ),
        LoginResult::Cancelled => (
            "cancelled",
            "×",
            "Login cancelled",
            "This login attempt was cancelled. Return to Agent to start again.",
        ),
        LoginResult::Failed => (
            "failed",
            "!",
            "Login failed",
            "Return to Agent to try connecting your ChatGPT account again.",
        ),
        LoginResult::Pending => (
            "pending",
            "…",
            "Login in progress",
            "Return to Agent to check this login attempt.",
        ),
    };
    let body = include_str!("pages/login_result.html")
        .replace("{{state}}", state)
        .replace("{{symbol}}", symbol)
        .replace("{{title}}", title)
        .replace("{{message}}", message);
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    )
}

fn parse_callback(
    path: &str,
    redirect_uri: &str,
    state: &str,
    client_id: &str,
) -> io::Result<Callback> {
    let base = Url::parse(redirect_uri).map_err(io::Error::other)?;
    let callback = base.join(path).map_err(io::Error::other)?;
    if callback.origin() != base.origin() || callback.path() != base.path() {
        return Err(io::Error::other("Unexpected callback address."));
    }
    let mut value_by_key = std::collections::HashMap::new();
    for (key, value) in callback.query_pairs() {
        if value_by_key
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(io::Error::other("Duplicate callback parameter."));
        }
    }
    if value_by_key.get("state").map(String::as_str) != Some(state) {
        return Err(io::Error::other("Login state mismatch."));
    }
    if value_by_key.contains_key("error") {
        let message = if value_by_key.get("error").map(String::as_str) == Some("access_denied") {
            "Login cancelled."
        } else {
            "ChatGPT login was declined or failed."
        };
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, message));
    }
    let issued_id = value_by_key
        .get("client_id")
        .map(String::as_str)
        .unwrap_or(client_id);
    if issued_id.is_empty()
        || issued_id == "dynamic_agent_client"
        || (client_id != "dynamic_agent_client" && issued_id != client_id)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "ChatGPT registration did not return the expected client ID.",
        ));
    }
    let code = value_by_key
        .get("code")
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Missing login code."))?;
    Ok(Callback {
        code: code.clone(),
        client_id: issued_id.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_releases_waiter_without_closing_the_server() {
        let server = CallbackServer::start().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let attempt = server.begin("oaiapp_test", cancelled.clone());
        let redirect_uri = attempt.redirect_uri.clone();
        let state = attempt.state.clone();
        let (finished, result) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            finished
                .send(attempt.receive().err().unwrap().to_string())
                .unwrap();
        });
        cancelled.store(true, Ordering::Relaxed);
        assert_eq!(
            result.recv_timeout(Duration::from_secs(1)).unwrap(),
            "Login cancelled."
        );
        worker.join().unwrap();

        let mut browser = TcpStream::connect(server.address).unwrap();
        browser
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            browser,
            "GET {redirect_uri}?state={state}&code=unused HTTP/1.1\r\n\r\n"
        )
        .unwrap();
        assert!(read_page(browser).contains("Login cancelled"));
    }

    #[test]
    fn late_cancelled_callback_does_not_affect_the_next_login() {
        let server = CallbackServer::start().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let first = server.begin("oaiapp_test", cancelled.clone());
        cancelled.store(true, Ordering::Relaxed);
        assert_eq!(
            first.receive().err().unwrap().to_string(),
            "Login cancelled."
        );
        let second = server.begin("oaiapp_test", Arc::new(AtomicBool::new(false)));
        assert_eq!(first.redirect_uri, second.redirect_uri);

        let page = read_page(connect(&first, "code=unused"));
        assert!(page.contains("Login cancelled"));
        assert!(second.receiver.try_recv().is_err());

        let browser = connect(&second, "code=second-code");
        let (callback, mut stream) = second.receive().unwrap();
        assert_eq!(callback.code, "second-code");
        second.finish(&mut stream, true);
        drop(stream);
        assert!(read_page(browser).contains("Agent connected"));
    }

    #[test]
    fn callback_is_delivered_once_and_reloads_show_its_result() {
        let server = CallbackServer::start().unwrap();
        for connected in [true, false] {
            let attempt = server.begin("oaiapp_test", Arc::new(AtomicBool::new(false)));
            let mut browser = connect(&attempt, "code=private-code");
            let (_, mut stream) = attempt.receive().unwrap();
            browser.set_nonblocking(true).unwrap();
            assert_eq!(
                browser.read(&mut [0; 1]).unwrap_err().kind(),
                io::ErrorKind::WouldBlock
            );
            browser.set_nonblocking(false).unwrap();

            let pending = read_page(connect(&attempt, "code=private-code"));
            assert!(pending.contains("Login in progress"));
            assert!(attempt.receiver.try_recv().is_err());

            attempt.finish(&mut stream, connected);
            drop(stream);
            let page = read_page(browser);
            assert_eq!(page.contains("Agent connected"), connected);
            assert_eq!(page.contains("Login failed"), !connected);
            assert!(!page.contains("private-code"));
            assert!(page.contains("Cache-Control: no-store"));
            assert_eq!(page, read_page(connect(&attempt, "code=private-code")));
        }
    }

    #[test]
    fn cancellation_after_delivery_returns_a_cancelled_page() {
        let server = CallbackServer::start().unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let attempt = server.begin("oaiapp_test", cancelled.clone());
        let browser = connect(&attempt, "code=unused");
        let (_, mut stream) = attempt.receive().unwrap();
        cancelled.store(true, Ordering::Relaxed);
        attempt.finish(&mut stream, false);
        drop(stream);
        assert!(read_page(browser).contains("Login cancelled"));
        assert!(read_page(connect(&attempt, "code=unused")).contains("Login cancelled"));
    }

    #[test]
    fn browser_denial_returns_cancelled_and_server_stays_available() {
        let server = CallbackServer::start().unwrap();
        let attempt = server.begin("oaiapp_test", Arc::new(AtomicBool::new(false)));
        let browser = connect(&attempt, "error=access_denied");
        assert_eq!(
            attempt.receive().err().unwrap().to_string(),
            "Login cancelled."
        );
        assert!(read_page(browser).contains("Login cancelled"));
        assert!(read_page(connect(&attempt, "code=late")).contains("Login cancelled"));
    }

    #[test]
    fn malformed_callback_does_not_consume_a_pending_attempt() {
        let server = CallbackServer::start().unwrap();
        let attempt = server.begin("oaiapp_test", Arc::new(AtomicBool::new(false)));
        assert!(
            read_page(connect(&attempt, "state=duplicate&code=unused")).contains("Login failed")
        );
        assert!(attempt.receiver.try_recv().is_err());
        let browser = connect(&attempt, "code=valid");
        let (callback, mut stream) = attempt.receive().unwrap();
        assert_eq!(callback.code, "valid");
        attempt.finish(&mut stream, true);
        drop(stream);
        assert!(read_page(browser).contains("Agent connected"));
    }

    #[test]
    fn dropping_attempt_keeps_cancelled_page_until_server_shutdown() {
        let server = CallbackServer::start().unwrap();
        let attempt = server.begin("oaiapp_test", Arc::new(AtomicBool::new(false)));
        let browser = connect(&attempt, "code=unused");
        drop(attempt);
        assert!(read_page(browser).contains("Login cancelled"));
        let address = server.address;
        drop(server);
        assert!(TcpStream::connect(address).is_err());
    }

    fn connect(attempt: &LoginAttempt, query: &str) -> TcpStream {
        let url = Url::parse(&attempt.redirect_uri).unwrap();
        let mut stream = TcpStream::connect(("127.0.0.1", url.port().unwrap())).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "GET /auth/callback?state={}&{query} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
            attempt.state
        )
        .unwrap();
        stream
    }

    fn read_page(mut stream: TcpStream) -> String {
        let mut page = String::new();
        stream.read_to_string(&mut page).unwrap();
        page
    }
    #[test]
    fn callback_requires_matching_state_origin_and_client() {
        let redirect = "http://127.0.0.1:1455/auth/callback";
        let callback = parse_callback(
            "/auth/callback?state=expected&code=code&client_id=oaiapp_test",
            redirect,
            "expected",
            "dynamic_agent_client",
        )
        .unwrap();
        assert_eq!(callback.client_id, "oaiapp_test");

        for path in [
            "/auth/callback?state=wrong&code=code&client_id=oaiapp_test",
            "/auth/callback?state=expected&state=expected&code=code&client_id=oaiapp_test",
            "http://elsewhere.test/auth/callback?state=expected&code=code&client_id=oaiapp_test",
            "/auth/callback?state=expected&code=code",
        ] {
            assert!(parse_callback(path, redirect, "expected", "dynamic_agent_client").is_err());
        }
        assert!(
            parse_callback(
                "/auth/callback?state=expected&code=code&client_id=other",
                redirect,
                "expected",
                "oaiapp_test",
            )
            .is_err()
        );
    }

    #[test]
    fn declined_login_does_not_exchange_a_code() {
        let error = parse_callback(
            "/auth/callback?state=expected&error=access_denied&code=ignored",
            "http://127.0.0.1:1455/auth/callback",
            "expected",
            "dynamic_agent_client",
        )
        .err()
        .unwrap();
        assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    }
}
