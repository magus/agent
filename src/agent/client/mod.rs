#[cfg(not(target_os = "ios"))]
mod args;
pub(crate) mod chat;
pub(crate) mod chatgpt;

#[cfg(not(target_os = "ios"))]
pub(crate) use args::Args;
