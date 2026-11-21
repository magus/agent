use crate::protocol::ModelSettings;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::Mutex;

pub(super) struct MessageDelivery {
    path: PathBuf,
    pending: Mutex<Option<Delivery>>,
}

impl MessageDelivery {
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let pending = match fs::read(&path) {
            Ok(bytes) => {
                let mut delivery: Delivery = serde_json::from_slice(&bytes)?;
                delivery.failure = Some("Delivery not confirmed. Retry this message.".into());
                Some(delivery)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };

        Ok(Self {
            path,
            pending: Mutex::new(pending),
        })
    }

    pub fn pending(&self) -> Option<Delivery> {
        self.pending.lock().unwrap().clone()
    }

    pub fn begin(&self, proposed: Delivery) -> io::Result<Delivery> {
        let mut pending = self.pending.lock().unwrap();
        let mut delivery = proposed;

        if let Some(existing) = pending.as_ref() {
            if existing.service_host != delivery.service_host
                || existing.request.id != delivery.request.id
                || existing.request.message_id != delivery.request.message_id
            {
                return Err(io::Error::other(
                    "Retry the pending message before sending a different one.",
                ));
            }
            delivery = existing.clone();
        }

        delivery.failure = None;
        super::chat_client::save_json(&self.path, &delivery)?;
        *pending = Some(delivery.clone());
        Ok(delivery)
    }

    pub fn accepted(&self) -> io::Result<()> {
        let mut pending = self.pending.lock().unwrap();
        fs::remove_file(&self.path)?;
        *pending = None;
        Ok(())
    }

    pub fn failed(&self, error: &io::Error) {
        if let Some(pending) = self.pending.lock().unwrap().as_mut() {
            pending.failure = Some(error.to_string());
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Delivery {
    pub service_host: String,
    pub request: DeliveryRequest,
    #[serde(default)]
    pub failure: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct DeliveryRequest {
    pub action: String,
    pub id: String,
    pub message_id: String,
    pub prompt: String,
    #[serde(flatten)]
    pub settings: ModelSettings,
}
