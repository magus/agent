use super::device_pairing::Invitation;
use super::device_service::DeviceService;
use super::remote_request::*;
use crate::devices::device_store::DeviceStore;
use crate::devices::identity::{Identity, device_id};
use crate::devices::tool_store::{Tool, ToolStore};
use crate::devices::transport;
use base64::Engine;
use serde_json::json;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

fn device(name: &str) -> Arc<DeviceService> {
    device_with_kind(name, true)
}

pub(crate) fn device_with_kind(name: &str, execution_device: bool) -> Arc<DeviceService> {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join(".scratch/2026-11-08-03-10-device-tools")
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&directory).unwrap();
    let probe = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = probe.local_addr().unwrap();
    drop(probe);
    let service = Arc::new(DeviceService {
        execution_device,
        port: address.port(),
        identity: Identity::generate().unwrap(),
        store: Mutex::new(DeviceStore::default()),
        store_path: directory.join("devices.json"),
        invitation_by_secret: Mutex::new(HashMap::new()),
        pending_by_id: Mutex::new(HashMap::new()),
        listening: Mutex::new(None),
        candidate_by_id: Mutex::new(HashMap::new()),
        status_by_id: Mutex::new(HashMap::new()),
        connection_status: Mutex::new(String::new()),
        tools: ToolStore::load(directory).unwrap(),
        service_handler: Mutex::new(None),
    });
    service.configure(address, name.into()).unwrap();
    service
}

#[test]
fn device_view_preserves_directional_access_after_revocation() {
    let first = device("first");
    let second = device("second");
    pair(&first, &second);
    pair(&second, &first);
    let second_id = device_id(&second.identity.certificate);

    first.revoke_access(&second_id).unwrap();
    let view = serde_json::to_value(first.view()).unwrap();
    let connection = &view["device_list"][0];
    assert_eq!(view["device_list"].as_array().unwrap().len(), 1);
    assert_eq!(connection["id"], second_id);
    assert_eq!(connection["name"], "second");
    assert_eq!(connection["outgoing"], true);
    assert_eq!(connection["incoming"], false);
    assert_eq!(connection["blocked"], true);
    assert!(connection.get("certificate").is_none());
}

#[test]
fn central_endpoint_requires_pairing_and_does_not_expose_local_tools() {
    let client = device("client");
    let central = device_with_kind("central", false);
    *central.service_handler.lock().unwrap() =
        Some(Arc::new(|sender, _| Ok(json!({"sender":sender.id}))));
    let central_id = device_id(&central.identity.certificate);
    assert!(
        client
            .call(
                &central_id,
                RemoteRequest::Service {
                    request: json!({"action":"list"})
                }
            )
            .is_err()
    );
    pair(&client, &central);
    let result = client
        .call(
            &central_id,
            RemoteRequest::Service {
                request: json!({"action":"list"}),
            },
        )
        .unwrap();
    assert_eq!(result["sender"], device_id(&client.identity.certificate));
    let shell = Tool::Shell {
        command: "echo forbidden".into(),
        cwd: "/".into(),
        timeout_seconds: 10,
    };
    assert!(
        client
            .call(
                &central_id,
                RemoteRequest::Submit {
                    id: uuid::Uuid::new_v4().to_string(),
                    tool: shell
                }
            )
            .is_err()
    );
    let connection = super::super::ToolConnection::new(central.clone());
    assert!(connection.target_list().is_empty());
    assert!(connection.info(&central_id).is_err());
    central
        .revoke_access(&device_id(&client.identity.certificate))
        .unwrap();
    assert!(
        client
            .call(
                &central_id,
                RemoteRequest::Service {
                    request: json!({"action":"list"})
                }
            )
            .is_err()
    );
}

fn pair(controller: &Arc<DeviceService>, target: &DeviceService) {
    let invitation = target.invite().unwrap();
    controller
        .pair(invitation["invitation"].as_str().unwrap())
        .unwrap();
    target
        .approve(&device_id(&controller.identity.certificate))
        .unwrap();
}

#[test]
fn removing_one_direction_preserves_the_other() {
    let first = device("first");
    let second = device("second");
    pair(&first, &second);
    pair(&second, &first);
    let first_id = device_id(&first.identity.certificate);
    let second_id = device_id(&second.identity.certificate);

    let connection = crate::devices::ToolConnection::new(first.clone());
    assert_eq!(connection.local_device().id, first_id);
    assert_eq!(connection.local_device().name, "first");
    assert!(
        connection
            .target_list()
            .iter()
            .any(|device| device.id == second_id)
    );

    first.revoke_access(&second_id).unwrap();
    assert!(first.call("second", RemoteRequest::Info).is_ok());
    assert!(second.call("first", RemoteRequest::Info).is_err());

    assert!(first.approve(&second_id).is_err());
    first.allow_requests(&second_id).unwrap();
    pair(&second, &first);
    first.remove_target(&second_id).unwrap();
    let target_list = connection.target_list();
    assert_eq!(target_list.len(), 1);
    assert_eq!(target_list[0].id, first_id);
    assert!(first.call("second", RemoteRequest::Info).is_err());
    assert!(second.call("first", RemoteRequest::Info).is_ok());
    let saved = DeviceStore::load(&first.store_path).unwrap();
    assert!(saved.peer_list[0].may_control_us);
    assert!(!saved.peer_list[0].target);
    assert!(second.store.lock().unwrap().peer(&first_id).is_ok());
}

#[test]
fn declined_requests_stay_declined_after_retry_and_reload() {
    let first = device("first");
    let second = device("second");
    let invitation = second.invite().unwrap();
    let invitation = invitation["invitation"].as_str().unwrap();
    let id = device_id(&first.identity.certificate);
    first.pair(invitation).unwrap();
    first.pair(invitation).unwrap();
    assert_eq!(second.pending_by_id.lock().unwrap().len(), 1);

    second.decline(&id).unwrap();
    *second.store.lock().unwrap() = DeviceStore::load(&second.store_path).unwrap();
    assert_eq!(first.pair(invitation).unwrap()["status"], "Declined");
    assert!(second.pending_by_id.lock().unwrap().is_empty());
    assert!(first.call("second", RemoteRequest::Info).is_err());
}

#[test]
fn pairing_requires_the_requesting_endpoint_to_hold_its_private_key() {
    let first = device("first");
    let impostor = device("impostor");
    let mut claimed = first.local_device().unwrap();
    claimed.address = impostor.local_device().unwrap().address;
    assert!(first.verify_device(claimed).is_err());
    assert!(
        first
            .dispatch(
                None,
                RemoteRequest::RequestAccess {
                    device: impostor.local_device().unwrap(),
                }
            )
            .is_err()
    );
}

#[test]
fn pairing_is_directional_persistent_and_revocable() {
    let controller = device("controller");
    let target = device("target");
    let invitation = target.invite().unwrap();
    controller
        .pair(invitation["invitation"].as_str().unwrap())
        .unwrap();
    assert!(controller.call("target", RemoteRequest::Info).is_err());
    let controller_id = device_id(&controller.identity.certificate);
    target.approve(&controller_id).unwrap();
    assert_eq!(
        controller.call("target", RemoteRequest::Info).unwrap()["tool_list"],
        json!(["shell"])
    );
    assert!(target.call("controller", RemoteRequest::Info).is_err());
    *target.store.lock().unwrap() = DeviceStore::load(&target.store_path).unwrap();
    *controller.store.lock().unwrap() = DeviceStore::load(&controller.store_path).unwrap();
    assert!(controller.call("target", RemoteRequest::Info).is_ok());
    target.revoke_access(&controller_id).unwrap();
    assert!(controller.call("target", RemoteRequest::Info).is_err());
}

#[test]
fn anonymous_wrong_identity_and_wrong_server_are_rejected() {
    let controller = device("controller");
    let target = device("target");
    pair(&controller, &target);
    let address = target.local_device().unwrap().address;
    let mut anonymous =
        transport::connect(address, target.identity.certificate.clone(), None).unwrap();
    transport::send(&mut anonymous, &RemoteRequest::Info).unwrap();
    assert!(read_result(&mut anonymous).is_err());

    let impostor = Identity::generate().unwrap();
    let mut connection = transport::connect(
        address,
        target.identity.certificate.clone(),
        Some(&impostor),
    )
    .unwrap();
    let accepted = transport::send(&mut connection, &RemoteRequest::Info)
        .and_then(|_| read_result(&mut connection));
    assert!(accepted.is_err());

    let mut wrong_server = transport::connect(
        address,
        impostor.certificate.clone(),
        Some(&controller.identity),
    )
    .unwrap();
    assert!(transport::send(&mut wrong_server, &RemoteRequest::Info).is_err());
}

#[test]
fn invitations_expire_and_cannot_be_claimed_by_another_device() {
    let first = device("first");
    let second = device("second");
    let target = device("target");
    let invitation = target.invite().unwrap();
    let invitation = invitation["invitation"].as_str().unwrap();
    first.pair(invitation).unwrap();
    assert!(second.pair(invitation).is_err());
    for entry in target.invitation_by_secret.lock().unwrap().values_mut() {
        entry.created = Instant::now() - Duration::from_secs(601);
    }
    assert!(first.pair(invitation).is_err());
}

#[test]
fn malformed_certificate_cannot_poison_pairing() {
    let controller = device("controller");
    let target = device("target");
    let invitation = target.invite().unwrap();
    let encoded = invitation["invitation"].as_str().unwrap();
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .unwrap();
    let invitation: Invitation = serde_json::from_slice(&bytes).unwrap();
    let mut device = controller.local_device().unwrap();
    device.certificate = vec![1, 2, 3];
    assert!(
        target
            .dispatch(
                None,
                RemoteRequest::Pair {
                    secret: invitation.secret,
                    device
                }
            )
            .is_err()
    );
    assert!(target.pending_by_id.lock().unwrap().is_empty());
    controller.pair(encoded).unwrap();
}

#[test]
fn remote_shell_survives_disconnect_and_duplicate_submission() {
    let controller = device("controller");
    let target = device("target");
    pair(&controller, &target);
    let id = uuid::Uuid::new_v4().to_string();
    let directory = target.store_path.parent().unwrap().to_path_buf();
    let tool = Tool::Shell {
        command: "sleep 0.2; printf x >> count; printf done".into(),
        cwd: directory.clone(),
        timeout_seconds: 10,
    };
    let submit = || RemoteRequest::Submit {
        id: id.clone(),
        tool: tool.clone(),
    };
    assert_eq!(
        controller.call("target", submit()).unwrap()["status"],
        "running"
    );
    controller.call("target", submit()).unwrap();
    let started = Instant::now();
    loop {
        let job = controller
            .call("target", RemoteRequest::Status { id: id.clone() })
            .unwrap();
        if job["status"] != "running" {
            assert_eq!(job["result"]["stdout"], "done");
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(25));
    }
    controller.call("target", submit()).unwrap();
    assert_eq!(std::fs::read(directory.join("count")).unwrap(), b"x");
    let reloaded = ToolStore::load(directory).unwrap();
    let job = reloaded
        .get(&device_id(&controller.identity.certificate), &id)
        .unwrap();
    assert_eq!(job.status, "completed");
    assert!(reloaded.get("another-device", &id).is_err());
    assert!(
        controller
            .call(
                "target",
                RemoteRequest::Submit {
                    id,
                    tool: Tool::Shell {
                        command: "true".into(),
                        cwd: PathBuf::from("/tmp"),
                        timeout_seconds: 1
                    }
                }
            )
            .is_err()
    );
}

#[test]
fn remote_shell_can_be_cancelled_and_revocation_blocks_new_tools() {
    let controller = device("controller");
    let target = device("target");
    pair(&controller, &target);
    let id = uuid::Uuid::new_v4().to_string();
    let tool = Tool::Shell {
        command: "sleep 30".into(),
        cwd: PathBuf::from("/tmp"),
        timeout_seconds: 60,
    };
    controller
        .call(
            "target",
            RemoteRequest::Submit {
                id: id.clone(),
                tool: tool.clone(),
            },
        )
        .unwrap();
    controller
        .call("target", RemoteRequest::Cancel { id: id.clone() })
        .unwrap();
    let started = Instant::now();
    loop {
        let job = controller
            .call("target", RemoteRequest::Status { id: id.clone() })
            .unwrap();
        if job["status"] != "running" {
            assert_eq!(job["status"], "cancelled");
            break;
        }
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::sleep(Duration::from_millis(25));
    }
    target
        .revoke_access(&device_id(&controller.identity.certificate))
        .unwrap();
    assert!(
        controller
            .call(
                "target",
                RemoteRequest::Submit {
                    id: uuid::Uuid::new_v4().to_string(),
                    tool
                }
            )
            .is_err()
    );
}
