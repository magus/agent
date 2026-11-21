use super::device_service::{DeviceService, RemoteRequest};
#[cfg(not(target_os = "ios"))]
use super::tool_store::Tool;
use crate::protocol::Target;
use serde_json::Value;
use std::io;
use std::sync::Arc;

pub(crate) struct ToolConnection {
    service: Arc<DeviceService>,
}

impl ToolConnection {
    pub(crate) fn new(service: Arc<DeviceService>) -> Arc<Self> {
        Arc::new(Self { service })
    }

    pub(crate) fn service_request(&self, host: &str, request: Value) -> io::Result<Value> {
        self.service.call(host, RemoteRequest::Service { request })
    }

    pub(crate) fn local_device(&self) -> Target {
        self.service.local_target()
    }

    pub(crate) fn target_list(&self) -> Vec<Target> {
        self.service.target_list()
    }

    pub(crate) fn info(&self, host: &str) -> io::Result<Value> {
        self.call(host, RemoteRequest::Info)
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn validate(&self, host: &str, tool: &Value) -> io::Result<()> {
        if !self.target_list().iter().any(|target| target.id == host) {
            return Err(io::Error::other(
                "Unknown or unauthorized target. Use a listed host ID.",
            ));
        }
        let tool: Tool = serde_json::from_value(tool.clone())?;
        tool.validate()
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn submit(&self, host: &str, id: &str, tool: Value) -> io::Result<Value> {
        let tool: Tool = serde_json::from_value(tool)?;
        self.call(
            host,
            RemoteRequest::Submit {
                id: id.into(),
                tool,
            },
        )
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn status(&self, host: &str, id: &str) -> io::Result<Value> {
        self.call(host, RemoteRequest::Status { id: id.into() })
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn status_for(&self, host: &str, id: &str, owner: &str) -> io::Result<Value> {
        let local = self.local_device();
        if host == local.id && !owner.is_empty() {
            // Preserve access to a saved local request without submitting it again.
            return serde_json::to_value(self.service.tools.get(owner, id)?)
                .map_err(io::Error::other);
        }
        if !owner.is_empty() && owner != local.id {
            return Err(io::Error::other(
                "This remote result belongs to the previous execution host. No action was repeated.",
            ));
        }
        self.status(host, id)
    }

    #[cfg(not(target_os = "ios"))]
    pub(crate) fn cancel(&self, host: &str, id: &str) -> io::Result<Value> {
        self.call(host, RemoteRequest::Cancel { id: id.into() })
    }

    fn call(&self, host: &str, request: RemoteRequest) -> io::Result<Value> {
        let local = self.service.local_target();
        if local.id != host {
            return self.service.call(host, request);
        }
        if !self.service.execution_device {
            return Err(io::Error::other(
                "The central service has no local execution tools.",
            ));
        }
        match request {
            RemoteRequest::Info => {
                Ok(serde_json::json!({"name": local.name, "tool_list": self.service.tool_list()}))
            }
            RemoteRequest::Submit { id, tool } => {
                serde_json::to_value(self.service.tools.submit(host, &id, tool)?)
                    .map_err(io::Error::other)
            }
            RemoteRequest::Status { id } => {
                serde_json::to_value(self.service.tools.get(host, &id)?).map_err(io::Error::other)
            }
            RemoteRequest::Cancel { id } => {
                serde_json::to_value(self.service.tools.cancel(host, &id)?)
                    .map_err(io::Error::other)
            }
            _ => Err(io::Error::other("Unsupported tool request")),
        }
    }
}
