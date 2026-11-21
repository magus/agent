use super::message_delivery::{Delivery, DeliveryRequest, MessageDelivery};
use crate::protocol::ModelSettings;
use std::fs;
use std::io;

#[test]
fn retry_retains_the_original_request_after_failure_and_restart() {
    let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("pending_message.json");
    let delivery = MessageDelivery::load(path.clone()).unwrap();
    delivery.begin(message()).unwrap();
    delivery.failed(&io::Error::other("Connection lost"));
    assert_eq!(
        delivery.pending().unwrap().failure.as_deref(),
        Some("Connection lost")
    );

    let reopened = MessageDelivery::load(path.clone()).unwrap();
    assert!(reopened.pending().unwrap().failure.is_some());
    let mut retry = message();
    retry.request.prompt = "Changed text must not replace the stored message".into();
    retry.request.settings.model = "different-model".into();
    let retained = reopened.begin(retry).unwrap();
    assert_eq!(retained.request.prompt, "Hello");
    assert_eq!(retained.request.settings.model, "model");
    assert_eq!(retained.request.message_id, "receipt");
    assert!(retained.failure.is_none());
    assert!(path.exists());

    reopened.accepted().unwrap();
    assert!(reopened.pending().is_none());
    assert!(!path.exists());
    assert!(MessageDelivery::load(path).unwrap().pending().is_none());
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn another_message_cannot_replace_unconfirmed_delivery() {
    let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    fs::create_dir_all(&directory).unwrap();
    let delivery = MessageDelivery::load(directory.join("pending_message.json")).unwrap();
    delivery.begin(message()).unwrap();

    for field in ["service", "conversation", "receipt"] {
        let mut other = message();
        match field {
            "service" => other.service_host = "another-service".into(),
            "conversation" => other.request.id = "another-chat".into(),
            "receipt" => other.request.message_id = "another-receipt".into(),
            _ => unreachable!(),
        }
        assert!(delivery.begin(other).is_err());
        assert_eq!(delivery.pending().unwrap().request.message_id, "receipt");
    }
    fs::remove_dir_all(directory).unwrap();
}

fn message() -> Delivery {
    Delivery {
        service_host: "service".into(),
        request: DeliveryRequest {
            action: "send".into(),
            id: "chat".into(),
            message_id: "receipt".into(),
            prompt: "Hello".into(),
            settings: ModelSettings {
                model: "model".into(),
                reasoning_effort: Some("low".into()),
            },
        },
        failure: None,
    }
}
