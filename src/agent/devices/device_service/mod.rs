mod device_connection;
mod device_discovery;
mod device_pairing;
#[allow(clippy::module_inception)]
mod device_service;
mod device_view;
mod remote_request;

pub(crate) use device_service::DeviceService;
pub(super) use remote_request::RemoteRequest;
#[cfg(not(target_os = "ios"))]
pub(super) use remote_request::response;

#[cfg(test)]
mod device_service_tests;

#[cfg(test)]
pub(crate) use device_service_tests::device_with_kind;
