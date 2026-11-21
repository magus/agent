mod client;
mod devices;
mod native_bridge;
#[cfg(not(target_os = "ios"))]
mod platform;
mod protocol;
#[cfg(not(target_os = "ios"))]
mod run;
#[cfg(not(target_os = "ios"))]
mod service;

#[cfg(not(target_os = "ios"))]
pub use run::run;
