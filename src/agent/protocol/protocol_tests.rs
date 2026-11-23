use super::{ConversationResponse, LoginRequest, ServiceListing, ServiceRequest};
use serde_json::{Value, json};

#[test]
fn conversation_response_round_trips_history_and_tools() {
    let payload = json!({
        "conversation": {
            "id": "conversation-id",
            "model": "model-id",
            "entry_list": [{
                "message_id": null,
                "sender": null,
                "role": "Tool",
                "text": "hostname",
                "image": "capture.jpg",
                "request": {
                    "owner": "service", "id": "original-request-id", "call_id": "call-id",
                    "host": "mini", "tool": {"tool": "shell", "command": "hostname"},
                    "status": "running", "finished": false
                }
            }],
            "status": "Running"
        },
        "record": {
            "id": "conversation-id", "title": "Existing chat", "creator": "macbook",
            "allowed_target_list": ["mini"], "last_activity": 1795434000
        },
        "running": true, "revision": 5, "image_by_id": {"capture.jpg": "image-bytes"},
        "target_list": [{"id": "mini", "name": "Mac mini"}]
    });

    let response: ConversationResponse = serde_json::from_value(payload.clone()).unwrap();

    let request = response.conversation.entry_list[0]
        .request
        .as_ref()
        .unwrap();

    assert!(!request.finished);
    assert_eq!(serde_json::to_value(response).unwrap(), payload);
}

#[test]
fn service_catalog_round_trips_grants() {
    let payload = json!({
        "conversation_list": [{
            "id": "chat", "title": "Existing chat", "creator": "macbook",
            "allowed_target_list": ["mini"], "last_activity": 1795434000
        }],
        "target_list": [{"id": "mini", "name": "Mac mini"}],
        "default_target_list": ["mini"],
        "service": {"id": "service", "name": "Agent service"}
    });

    let listing: ServiceListing = serde_json::from_value(payload.clone()).unwrap();

    assert_eq!(serde_json::to_value(listing).unwrap(), payload);
}

#[test]
fn service_and_login_requests_validate_and_round_trip() {
    let request_list = [
        json!({"action":"list"}),
        json!({"action":"send", "id":"chat", "message_id":"message", "prompt":"hello", "model":"model", "reasoning_effort":null}),
        json!({"action":"get", "id":"chat", "known_image_list":[]}),
        json!({"action":"set_targets", "id":"chat", "target_list":["mini"]}),
        json!({"action":"set_defaults", "target_list":["mini"]}),
        json!({"action":"login", "request":{"action":"registration"}}),
    ];
    for payload in request_list {
        let request: ServiceRequest = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(serde_json::to_value(request).unwrap(), payload);
    }

    let payload =
        json!({"action":"complete", "state":"state", "code":"test-code", "client_id":"client"});
    let request: LoginRequest = serde_json::from_value(payload.clone()).unwrap();
    assert_eq!(serde_json::to_value(request).unwrap(), payload);

    let get: ServiceRequest = serde_json::from_value(json!({"action":"get", "id":"chat"})).unwrap();
    assert_eq!(
        serde_json::to_value(get).unwrap()["known_image_list"],
        json!([])
    );
    assert!(
        serde_json::from_value::<ServiceRequest>(
            json!({"action":"get", "id":"chat", "unexpected":true})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<LoginRequest>(Value::Null).is_err());
}
