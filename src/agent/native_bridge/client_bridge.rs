use super::login_session::LoginSession;
use crate::client::chat::{ChatClient, Delivery, DeliveryRequest, conversation_notice};
use crate::devices::{DeviceService, ToolConnection};
use crate::protocol::ModelSettings;
use serde::Deserialize;
use serde_json::{Value, json};
use std::ffi::{CStr, CString, c_char};
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};

static CLIENT: OnceLock<NativeClient> = OnceLock::new();

struct NativeClient {
    chat: Arc<ChatClient>,
    devices: Arc<DeviceService>,
    login: Arc<LoginSession>,
}

// Calls run on a worker queue. Returned strings belong to the caller until agent_string_free.
#[unsafe(no_mangle)]
unsafe extern "C" fn agent_request(request: *const c_char) -> *mut c_char {
    let result = std::panic::catch_unwind(|| {
        if request.is_null() {
            return Err(io::Error::other("Missing native request"));
        }

        let bytes = unsafe { CStr::from_ptr(request) }.to_bytes();
        let request = serde_json::from_slice(bytes)?;
        dispatch(request)
    });
    let response = match result {
        Ok(Ok(value)) => json!({"ok": true, "result": value}),
        Ok(Err(error)) => json!({"ok": false, "error": error.to_string()}),
        Err(_) => json!({"ok": false, "error": "Agent client failed"}),
    };

    CString::new(response.to_string()).unwrap().into_raw()
}

#[unsafe(no_mangle)]
unsafe extern "C" fn agent_string_free(value: *mut c_char) {
    if !value.is_null() {
        drop(unsafe { CString::from_raw(value) });
    }
}

fn dispatch(request: Request) -> io::Result<Value> {
    if let Request::Initialize { directory, name } = request {
        if CLIENT.get().is_some() {
            return Ok(json!({}));
        }

        let directory = PathBuf::from(directory);
        #[cfg(not(target_os = "ios"))]
        let devices = {
            let _ = name;
            DeviceService::load(directory.clone())?
        };
        #[cfg(target_os = "ios")]
        let devices = DeviceService::load_client(directory.clone(), name)?;
        #[cfg(not(target_os = "ios"))]
        crate::devices::start_admin(devices.clone(), &directory)?;
        let connection = ToolConnection::new(devices.clone());
        let chat = ChatClient::load(connection, directory.join("client"))?;
        let login = LoginSession::new(chat.clone());
        CLIENT
            .set(NativeClient {
                chat,
                devices,
                login,
            })
            .map_err(|_| io::Error::other("Client already initialized"))?;
        return Ok(json!({}));
    }

    let client = CLIENT
        .get()
        .ok_or_else(|| io::Error::other("Client is not initialized"))?;
    let chat = &client.chat;
    match request {
        Request::Snapshot {
            showing_device_chips,
        } => {
            let service_host = chat.service_host().unwrap_or_default();
            let available_target_list = chat.available_target_list();
            let local_device_id = client.devices.local_target().id;
            let unavailable_tools = visible_unavailable_tools(
                showing_device_chips && !service_host.is_empty(),
                &available_target_list,
                &local_device_id,
                || client.devices.unavailable_tools(),
            );
            Ok(json!({
                "conversation": chat.view(),
                "pending": chat.pending_delivery(),
                "notice": conversation_notice(&chat.view().status),
                "conversation_list": chat.conversation_list(),
                "service_list": chat.service_list(),
                "service_host": service_host,
                "running": chat.running(),
                "revision": chat.revision(),
                "target_list": chat.target_list(),
                "available_target_list": available_target_list,
                "default_target_list": chat.default_target_list(),
                "model_settings": chat.model_settings(),
                "local_device_id": local_device_id,
                "local_unavailable_tools": unavailable_tools,
            }))
        }
        Request::SearchSources {
            service,
            id,
            entry_index,
        } => chat.request_at(
            &service,
            json!({"action":"search_sources","id":id,"entry_index":entry_index}),
        ),
        Request::ConversationJson {
            service,
            id,
            events,
            decode_nested,
        } => super::conversation_json::load(chat, &service, &id, events, decode_nested),
        Request::Models => {
            serde_json::to_value(chat.model_option_list()?).map_err(io::Error::other)
        }
        Request::Connect { host } => {
            chat.connect(host)?;
            Ok(json!({}))
        }
        Request::Select { id } => {
            chat.select(id)?;
            Ok(json!({}))
        }
        Request::Create => {
            chat.create()?;
            Ok(json!({}))
        }
        Request::PrepareSend {
            service,
            id,
            prompt,
            settings,
        } => chat.prepare_delivery(Delivery {
            service_host: service,
            request: DeliveryRequest {
                action: "send".into(),
                id,
                message_id: uuid::Uuid::new_v4().to_string(),
                prompt,
                settings,
            },
            failure: None,
        }),
        Request::Send {
            service,
            id,
            message_id,
            prompt,
            settings,
        } => {
            chat.start(service, id, message_id, prompt, settings)?;
            Ok(json!({}))
        }
        Request::Retry => {
            chat.retry_delivery()?;
            Ok(json!({}))
        }
        Request::Stop => {
            chat.stop()?;
            Ok(json!({}))
        }
        Request::Retrieve => {
            chat.retrieve()?;
            Ok(json!({}))
        }
        Request::SetModel {
            service,
            id,
            settings,
        } => {
            chat.set_conversation_model(&service, &id, settings)?;
            Ok(json!({}))
        }
        Request::SetTargets {
            service,
            id,
            target_list,
        } => {
            chat.set_conversation_targets(&service, &id, target_list)?;
            Ok(json!({}))
        }
        Request::SetDeviceDefaults { target_list } => {
            chat.set_targets(target_list, true)?;
            Ok(json!({}))
        }
        Request::SetModelDefaults { settings } => {
            chat.set_model_settings(settings)?;
            Ok(json!({}))
        }
        Request::Pair { invitation } => client.devices.pair(&invitation),
        Request::Devices => Ok(client.devices.snapshot()),
        Request::Account { service, operation } => {
            let action = match operation {
                AccountAction::Status => "account",
                AccountAction::CheckModel => "check_model",
                AccountAction::SignOut => "sign_out",
            };
            chat.request_at(&service, json!({"action":action}))
        }
        Request::LoginStart { service, id } => {
            client.login.start(service, id)?;
            Ok(json!({}))
        }
        Request::LoginCancel => {
            client.login.cancel();
            Ok(json!({}))
        }
        Request::LoginStatus => serde_json::to_value(client.login.view()).map_err(io::Error::other),
        Request::DeviceView => {
            serde_json::to_value(client.devices.view()).map_err(io::Error::other)
        }
        Request::DeviceAction { request } => match request {
            DeviceAction::Rename { name } => client.devices.rename(name),
            DeviceAction::Refresh => {
                #[cfg(not(target_os = "ios"))]
                client.devices.discover()?;
                serde_json::to_value(client.devices.view()).map_err(io::Error::other)
            }
            DeviceAction::Find { host } => client.devices.find_device(&host),
            DeviceAction::RequestAccess { id } => client.devices.request_access(&id),
            DeviceAction::Approve { id } => client.devices.approve(&id),
            DeviceAction::Decline { id } => client.devices.decline(&id),
            DeviceAction::RevokeAccess { id } => client.devices.revoke_access(&id),
            DeviceAction::RemoveTarget { id } => client.devices.remove_target(&id),
            DeviceAction::AllowRequests { id } => client.devices.allow_requests(&id),
            DeviceAction::Invite => client.devices.invite(),
        },
        Request::Initialize { .. } => unreachable!(),
    }
}

#[derive(Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    Initialize {
        directory: String,
        name: String,
    },
    Snapshot {
        #[serde(default)]
        showing_device_chips: bool,
    },
    SearchSources {
        service: String,
        id: String,
        entry_index: usize,
    },
    ConversationJson {
        service: String,
        id: String,
        events: bool,
        #[serde(default)]
        decode_nested: bool,
    },
    Models,
    Connect {
        host: String,
    },
    Select {
        id: String,
    },
    Create,
    PrepareSend {
        service: String,
        id: String,
        prompt: String,
        settings: ModelSettings,
    },
    Send {
        service: String,
        id: String,
        message_id: String,
        prompt: String,
        settings: ModelSettings,
    },
    Retry,
    Stop,
    Retrieve,
    SetModel {
        service: String,
        id: String,
        settings: ModelSettings,
    },
    SetTargets {
        service: String,
        id: String,
        target_list: Vec<String>,
    },
    SetDeviceDefaults {
        target_list: Vec<String>,
    },
    SetModelDefaults {
        settings: ModelSettings,
    },
    Devices,
    Account {
        service: String,
        operation: AccountAction,
    },
    LoginStart {
        service: String,
        id: String,
    },
    LoginCancel,
    LoginStatus,
    DeviceView,
    DeviceAction {
        request: DeviceAction,
    },
    Pair {
        invitation: String,
    },
}

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
enum DeviceAction {
    Rename { name: String },
    Refresh,
    Find { host: String },
    RequestAccess { id: String },
    Approve { id: String },
    Decline { id: String },
    RevokeAccess { id: String },
    RemoveTarget { id: String },
    AllowRequests { id: String },
    Invite,
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AccountAction {
    Status,
    CheckModel,
    SignOut,
}

fn visible_unavailable_tools(
    showing_device_chips: bool,
    target_list: &[crate::protocol::Target],
    local_device_id: &str,
    check: impl FnOnce() -> Value,
) -> Value {
    let shows_local_chip = target_list
        .iter()
        .any(|target| target.id == local_device_id);
    if !showing_device_chips || !shows_local_chip {
        return json!({});
    }
    check()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_only_checks_permissions_for_a_visible_local_chip() {
        let local = crate::protocol::Target {
            id: "mac".into(),
            name: "Mac".into(),
        };
        let targets = [local];
        for (visible, list) in [(false, targets.as_slice()), (true, &[][..])] {
            let result =
                visible_unavailable_tools(visible, list, "mac", || panic!("unexpected probe"));
            assert_eq!(result, json!({}));
        }
        let result = visible_unavailable_tools(true, &targets, "other", || panic!("remote chip"));
        assert_eq!(result, json!({}));

        let reason = json!({"computer": "Permission missing"});
        assert_eq!(
            visible_unavailable_tools(true, &targets, "mac", || reason.clone()),
            reason
        );
    }
}
