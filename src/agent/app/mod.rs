#[allow(clippy::module_inception)]
mod app;
mod args;
mod chat;
mod chatgpt;
mod core;
pub(crate) mod desktop_service;
mod devices;

pub(crate) use app::App;
pub(crate) use app::Reopen;
pub(crate) use args::Args;
