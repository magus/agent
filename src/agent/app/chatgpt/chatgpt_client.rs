use super::callback_server::CallbackServer;
use super::login_timing::LoginTiming;
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use security_framework::passwords;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

pub(crate) struct ChatGptClient {
    http: ureq::Agent,
    host_id: String,
    session: Mutex<()>,
    callback_server: Mutex<Option<CallbackServer>>,
}

impl ChatGptClient {
    pub(crate) fn new(runtime_dir: &Path) -> io::Result<Self> {
        let host_path = runtime_dir.join("chatgpt-host-id");
        let host_id = match std::fs::read_to_string(&host_path) {
            Ok(value) => value,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let value = format!("urn:uuid:{}", uuid::Uuid::new_v4());
                std::fs::write(&host_path, &value)?;
                value
            }
            Err(error) => return Err(error),
        };
        let http = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(90)))
            .max_redirects(0)
            .build()
            .into();

        Ok(Self {
            http,
            host_id,
            session: Mutex::new(()),
            callback_server: Mutex::new(None),
        })
    }

    pub(crate) fn account(&self) -> io::Result<Option<String>> {
        let _session = self.session.lock().unwrap();
        Ok(load_account()?
            .filter(|account| account.tokens.is_some())
            .map(|account| account.label))
    }

    pub(super) fn sign_in(
        &self,
        cancelled: &Arc<AtomicBool>,
        timing: &LoginTiming,
    ) -> io::Result<String> {
        let _session = self.session.lock().unwrap();
        let previous = load_account()?;
        let client_id = previous
            .as_ref()
            .map_or("dynamic_agent_client", |account| &account.client_id);
        let attempt = {
            let mut server = self.callback_server.lock().unwrap();
            if server.is_none() {
                *server = Some(CallbackServer::start()?);
            }
            server.as_ref().unwrap().begin(client_id, cancelled.clone())
        };
        let redirect_uri = &attempt.redirect_uri;
        let state = &attempt.state;
        let nonce = random_value();
        let verifier = random_value();
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        let mut authorize = Url::parse("https://auth.openai.com/api/accounts/authorize").unwrap();
        authorize.query_pairs_mut().extend_pairs([
            ("client_id", client_id),
            ("ext_agent_host_id", &self.host_id),
            ("response_type", "code"),
            ("redirect_uri", redirect_uri),
            (
                "scope",
                "openid profile email offline_access resource.invoke chatgpt.tokens.use.direct",
            ),
            ("resource", "https://api.openai.com/v1"),
            ("state", state),
            ("nonce", &nonce),
            ("code_challenge_method", "S256"),
            ("code_challenge", &challenge),
        ]);
        if previous.is_none() {
            authorize
                .query_pairs_mut()
                .append_pair("agent_name_hint", "Agent");
        }
        let status = std::process::Command::new("/usr/bin/open")
            .arg(authorize.as_str())
            .status()?;
        if !status.success() {
            return Err(io::Error::other(
                "Could not open ChatGPT login in your browser.",
            ));
        }

        let (callback, mut stream) = attempt.receive()?;
        timing.mark("callback_received");
        let result = (|| {
            let response = self.token_request(&[
                ("grant_type", "authorization_code"),
                ("client_id", &callback.client_id),
                ("code", &callback.code),
                ("code_verifier", &verifier),
                ("redirect_uri", redirect_uri),
                ("resource", "https://api.openai.com/v1"),
            ])?;
            timing.mark("token_exchange_finished");
            let id_token = required_string(&response, "id_token")?;
            let identity = self.validate_identity(id_token, &callback.client_id, &nonce)?;
            timing.mark("identity_verified");
            if previous
                .as_ref()
                .is_some_and(|account| account.subject != identity.sub)
            {
                return Err(io::Error::other(
                    "This proof supports one saved account. Sign in with the original account.",
                ));
            }
            let tokens = read_tokens(&response, None)?;
            if cancelled.load(Ordering::Relaxed) {
                return Err(io::Error::other(
                    "Login cancelled. No credentials were saved.",
                ));
            }
            let account = SavedAccount {
                client_id: callback.client_id,
                subject: identity.sub.clone(),
                label: identity.email.unwrap_or(identity.sub),
                tokens: Some(tokens),
            };
            save_account(&account)?;
            timing.mark("credentials_saved");
            Ok(account.label)
        })();

        // A closed browser tab must not undo a saved login.
        attempt.finish(&mut stream, result.is_ok());
        timing.mark("browser_reply_finished");
        result
    }

    pub(crate) fn check_model(&self) -> io::Result<String> {
        let _session = self.session.lock().unwrap();
        let mut account =
            load_account()?.ok_or_else(|| io::Error::other("Sign in with ChatGPT first."))?;
        let tokens = account
            .tokens
            .as_ref()
            .ok_or_else(|| io::Error::other("Sign in with ChatGPT first."))?;
        if tokens.expires_at <= unix_time() + 60 {
            let response = self.token_request(&[
                ("grant_type", "refresh_token"),
                ("client_id", &account.client_id),
                ("refresh_token", &tokens.refresh_token),
                ("resource", "https://api.openai.com/v1"),
            ]);
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    if error.kind() == io::ErrorKind::PermissionDenied {
                        account.tokens = None;
                        save_account(&account)?;
                    }
                    return Err(error);
                }
            };
            account.tokens = Some(read_tokens(&response, Some(tokens))?);
            save_account(&account)?;
        }
        let token = &account.tokens.as_ref().unwrap().access_token;
        let authorization = format!("Bearer {token}");
        let mut response = self
            .http
            .get("https://api.openai.com/v1/models")
            .header("Authorization", &authorization)
            .call()
            .map_err(network_error)?;
        let catalog: Value = response.body_mut().read_json().map_err(network_error)?;
        let model = catalog["models"]
            .as_array()
            .and_then(|model_list| {
                model_list
                    .iter()
                    .find(|model| model["visibility"] == "list")
            })
            .and_then(|model| model["slug"].as_str())
            .ok_or_else(|| io::Error::other("ChatGPT returned no available models."))?;
        let request = json!({
            "model": model,
            "store": false,
            "stream": true,
            "input": [{"role": "user", "content": "Calculate 9 times 221. Call report_result with the integer answer."}],
            "tools": [{"type": "function", "name": "report_result", "description": "Report the arithmetic answer.",
                "strict": true, "parameters": {"type": "object", "properties": {"answer": {"type": "integer"}},
                "required": ["answer"], "additionalProperties": false}}],
            "tool_choice": {"type": "function", "name": "report_result"}
        });
        let mut response = self
            .http
            .post("https://api.openai.com/v1/responses")
            .header("Authorization", &authorization)
            .header("Accept", "text/event-stream")
            .send_json(&request)
            .map_err(network_error)?;
        let body = response
            .body_mut()
            .with_config()
            .limit(2 * 1024 * 1024)
            .read_to_string()
            .map_err(network_error)?;
        let completed = completed_response(&body)?;
        let output_list = completed["output"]
            .as_array()
            .ok_or_else(|| io::Error::other("The model returned no output."))?;
        let arguments = output_list
            .iter()
            .find(|item| item["type"] == "function_call" && item["name"] == "report_result")
            .and_then(|item| item["arguments"].as_str())
            .ok_or_else(|| io::Error::other("The model did not return the requested tool call."))?;
        let result: Value = serde_json::from_str(arguments).map_err(io::Error::other)?;
        if result["answer"].as_i64() != Some(1989) {
            return Err(io::Error::other(
                "The model completed, but its arithmetic result was incorrect.",
            ));
        }
        Ok(format!(
            "{model} returned 1989 through a completed tool call. ChatGPT login was used; no API key was sent."
        ))
    }

    pub(crate) fn sign_out(&self) -> io::Result<String> {
        let _session = self.session.lock().unwrap();
        let Some(mut account) = load_account()? else {
            return Ok("Signed out.".into());
        };
        let revoked = if let Some(tokens) = &account.tokens {
            self.revoke(&account.client_id, &tokens.refresh_token)
                .or_else(|_| {
                    std::thread::sleep(Duration::from_millis(500));
                    self.revoke(&account.client_id, &tokens.refresh_token)
                })
                .is_ok()
        } else {
            true
        };
        account.tokens = None;
        save_account(&account)?;
        Ok(if revoked { "Signed out." } else {
            "Signed out locally. Remote revocation was not confirmed. You can disconnect Agent in ChatGPT Settings."
        }.into())
    }

    fn token_request(&self, form: &[(&str, &str)]) -> io::Result<Value> {
        let mut response = self
            .http
            .post("https://auth.openai.com/api/accounts/oauth/token")
            .config()
            .http_status_as_error(false)
            .build()
            .send_form(form.iter().copied())
            .map_err(network_error)?;
        let status = response.status().as_u16();
        let body = response.body_mut().read_json().map_err(network_error)?;
        if !(200..300).contains(&status) {
            return Err(token_error(status, &body));
        }
        Ok(body)
    }

    fn validate_identity(&self, token: &str, client_id: &str, nonce: &str) -> io::Result<Identity> {
        let discovery = self.discovery()?;
        let uri = trusted_auth_url(&discovery, "jwks_uri")?;
        let mut response = self.http.get(uri).call().map_err(network_error)?;
        let key_set: JwkSet = response.body_mut().read_json().map_err(network_error)?;
        let header =
            decode_header(token).map_err(|_| io::Error::other("Invalid login identity."))?;
        let key_id = header
            .kid
            .ok_or_else(|| io::Error::other("Login identity has no signing key."))?;
        let key = key_set
            .find(&key_id)
            .ok_or_else(|| io::Error::other("Unknown login signing key."))?;
        let key = DecodingKey::from_jwk(key).map_err(io::Error::other)?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[client_id]);
        validation.set_issuer(&["https://auth.openai.com"]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        let identity = decode::<Identity>(token, &key, &validation)
            .map_err(|_| io::Error::other("ChatGPT identity validation failed."))?
            .claims;
        if identity.nonce != nonce || identity.sub.is_empty() {
            return Err(io::Error::other(
                "ChatGPT login did not match this attempt.",
            ));
        }
        Ok(identity)
    }

    fn discovery(&self) -> io::Result<Value> {
        let mut response = self
            .http
            .get("https://auth.openai.com/.well-known/openid-configuration")
            .call()
            .map_err(network_error)?;
        response.body_mut().read_json().map_err(network_error)
    }

    fn revoke(&self, client_id: &str, token: &str) -> io::Result<()> {
        let discovery = self.discovery()?;
        let uri = trusted_auth_url(&discovery, "revocation_endpoint")?;
        self.http
            .post(uri)
            .send_form([
                ("client_id", client_id),
                ("token", token),
                ("token_type_hint", "refresh_token"),
            ])
            .map_err(network_error)?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct SavedAccount {
    client_id: String,
    subject: String,
    label: String,
    tokens: Option<Tokens>,
}

#[derive(Serialize, Deserialize)]
struct Tokens {
    access_token: String,
    refresh_token: String,
    scope: String,
    expires_at: u64,
}

#[derive(Deserialize)]
struct Identity {
    sub: String,
    email: Option<String>,
    nonce: String,
}

fn read_tokens(response: &Value, previous: Option<&Tokens>) -> io::Result<Tokens> {
    let scope = response["scope"]
        .as_str()
        .or(previous.map(|tokens| tokens.scope.as_str()))
        .ok_or_else(|| io::Error::other("ChatGPT did not return granted permissions."))?;
    if !scope
        .split_whitespace()
        .any(|value| value == "chatgpt.tokens.use.direct")
    {
        return Err(io::Error::other(
            "ChatGPT plan usage was not granted. Enable it during login.",
        ));
    }
    if response["token_type"]
        .as_str()
        .is_none_or(|value| !value.eq_ignore_ascii_case("Bearer"))
    {
        return Err(io::Error::other("Unsupported ChatGPT token type."));
    }
    let lifetime = response["expires_in"]
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| io::Error::other("ChatGPT did not return a token lifetime."))?;
    let refresh_token = response["refresh_token"]
        .as_str()
        .or(previous.map(|tokens| tokens.refresh_token.as_str()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::other("ChatGPT did not return a refresh token."))?;
    Ok(Tokens {
        access_token: required_string(response, "access_token")?.into(),
        refresh_token: refresh_token.into(),
        scope: scope.into(),
        expires_at: unix_time().saturating_add(lifetime),
    })
}

fn completed_response(body: &str) -> io::Result<Value> {
    let normalized = body.replace("\r\n", "\n");
    for event in normalized.split("\n\n") {
        let data = event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim_start)
            .collect::<Vec<_>>()
            .join("\n");
        if data.is_empty() || data == "[DONE]" {
            continue;
        }
        let event: Value = serde_json::from_str(&data).map_err(io::Error::other)?;
        match event["type"].as_str() {
            Some("response.completed") if event["response"]["status"] == "completed" => {
                return Ok(event["response"].clone());
            }
            Some("response.failed" | "response.incomplete" | "error") => {
                return Err(io::Error::other(
                    "The ChatGPT request failed or reached a usage limit. Check ChatGPT Settings → Usage.",
                ));
            }
            _ => {}
        }
    }
    Err(io::Error::other(
        "The model stream ended without a completed response.",
    ))
}

fn load_account() -> io::Result<Option<SavedAccount>> {
    match passwords::get_generic_password("com.iamnoah.agent.chatgpt", "account") {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(io::Error::other),
        Err(error) if error.code() == -25300 => Ok(None),
        Err(_) => Err(io::Error::other(
            "Could not read Agent's ChatGPT account from Keychain.",
        )),
    }
}

fn save_account(account: &SavedAccount) -> io::Result<()> {
    let bytes = serde_json::to_vec(account).map_err(io::Error::other)?;
    passwords::set_generic_password("com.iamnoah.agent.chatgpt", "account", &bytes)
        .map_err(|_| io::Error::other("Could not save Agent's ChatGPT account in Keychain."))
}

fn required_string<'a>(value: &'a Value, field: &str) -> io::Result<&'a str> {
    value[field]
        .as_str()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| io::Error::other(format!("ChatGPT response is missing {field}.")))
}

fn trusted_auth_url<'a>(value: &'a Value, field: &str) -> io::Result<&'a str> {
    let text = required_string(value, field)?;
    let url = Url::parse(text).map_err(io::Error::other)?;
    if url.scheme() != "https" || url.host_str() != Some("auth.openai.com") {
        return Err(io::Error::other(
            "Unexpected OpenAI authentication endpoint.",
        ));
    }
    Ok(text)
}

fn random_value() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn network_error(error: impl std::fmt::Display) -> io::Error {
    // HTTP errors contain status codes, but never return response bodies or credentials to the UI.
    io::Error::other(format!("ChatGPT connection failed. {error}"))
}

fn token_error(status: u16, body: &Value) -> io::Error {
    let code = body["error"].as_str().or(body["error"]["code"].as_str());
    let session_expired = matches!(
        code,
        Some(
            "invalid_grant"
                | "invalid_refresh_token"
                | "token_expired"
                | "refresh_token_expired"
                | "refresh_token_invalidated"
                | "refresh_token_reused"
        )
    );
    if (400..500).contains(&status) && session_expired {
        return io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Your ChatGPT session has expired or was disconnected. Sign in again.",
        );
    }
    io::Error::other(format!(
        "ChatGPT authentication failed (HTTP {status}). Try again."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_only_discards_confirmed_unusable_tokens() {
        for code in [
            "invalid_grant",
            "invalid_refresh_token",
            "token_expired",
            "refresh_token_expired",
            "refresh_token_invalidated",
            "refresh_token_reused",
        ] {
            let error = token_error(400, &json!({"error": code}));
            assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
        }
        for (status, body) in [
            (503, json!({"error": "invalid_grant"})),
            (429, json!({"error": "rate_limit"})),
            (401, json!({"error": "invalid_client"})),
            (400, json!({"error_description": "private response"})),
        ] {
            let error = token_error(status, &body);
            assert_ne!(error.kind(), io::ErrorKind::PermissionDenied);
            assert!(!error.to_string().contains("private response"));
        }
    }

    #[test]
    fn stream_requires_completed_event_even_after_text() {
        let partial = "data: {\"type\":\"response.output_text.delta\",\"delta\":\"1989\"}\n\n";
        assert!(completed_response(partial).is_err());
        let failed = format!("{partial}data: {{\"type\":\"response.failed\"}}\n\n");
        assert!(completed_response(&failed).is_err());
        let completed = "data: {\"type\":\"response.completed\",\"response\":{\"status\":\"completed\",\"output\":[]}}\r\n\r\n";
        assert_eq!(
            completed_response(completed).unwrap()["status"],
            "completed"
        );
    }

    #[test]
    fn tokens_require_plan_scope_and_keep_rotated_refresh_token() {
        let mut response = json!({"access_token":"test-access", "refresh_token":"test-refresh",
            "token_type":"Bearer", "expires_in":3600, "scope":"openid"});
        assert!(read_tokens(&response, None).is_err());
        response["scope"] = json!("openid chatgpt.tokens.use.direct");
        let first = read_tokens(&response, None).unwrap();
        response["refresh_token"] = json!("test-rotated");
        let refreshed = read_tokens(&response, Some(&first)).unwrap();
        assert_eq!(refreshed.refresh_token, "test-rotated");
        response.as_object_mut().unwrap().remove("refresh_token");
        response.as_object_mut().unwrap().remove("scope");
        let retained = read_tokens(&response, Some(&refreshed)).unwrap();
        assert_eq!(retained.refresh_token, "test-rotated");
    }
}
