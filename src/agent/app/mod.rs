#[allow(clippy::module_inception)]
mod app;
mod args;
mod chatgpt;
mod core;
mod desktop_service;

pub(crate) use app::App;
use app::Reopen;
