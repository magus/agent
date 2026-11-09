use serde::Deserialize;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub(super) struct DesktopService {
    helper: PathBuf,
    pub(super) runtime_dir: PathBuf,
    child: Mutex<Option<Child>>,
}

impl DesktopService {
    pub(super) fn new() -> io::Result<Self> {
        let executable = std::env::current_exe()?;
        let helper = executable.parent().unwrap().join("desktop_service");
        let runtime_dir = crate::platform::runtime_dir()?;

        Ok(Self {
            helper,
            runtime_dir,
            child: Mutex::new(None),
        })
    }

    pub(super) fn start(&self) -> io::Result<()> {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(self.runtime_dir.join("service.log"))?;
        let child = Command::new(&self.helper)
            .arg(&self.runtime_dir)
            .arg("--serve")
            .stdin(Stdio::null())
            .stdout(log.try_clone()?)
            .stderr(log)
            .spawn()?;
        *self.child.lock().unwrap() = Some(child);
        Ok(())
    }

    pub(super) fn permissions(&self) -> io::Result<Permissions> {
        serde_json::from_value(self.call("--permissions")?).map_err(io::Error::other)
    }

    pub(super) fn call(&self, mode: &str) -> io::Result<Value> {
        let output = Command::new(&self.helper)
            .arg(&self.runtime_dir)
            .arg(mode)
            .stdin(Stdio::null())
            .output()?;
        let response: Value = serde_json::from_slice(&output.stdout).map_err(|error| {
            io::Error::other(format!(
                "Could not read the desktop service response. {error}"
            ))
        })?;
        if !output.status.success() || response["ok"] == false {
            let message = response["error"]
                .as_str()
                .unwrap_or("Desktop service failed");
            return Err(io::Error::other(message));
        }
        Ok(response)
    }

    pub(super) fn stop(&self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub(super) fn stopped(&self) -> io::Result<bool> {
        let mut child = self.child.lock().unwrap();
        match child.as_mut() {
            Some(child) => Ok(child.try_wait()?.is_some()),
            None => Ok(true),
        }
    }
}

impl Drop for DesktopService {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Clone, Default, Deserialize, PartialEq, Eq)]
pub(super) struct Permissions {
    pub(super) accessibility: bool,
    pub(super) screen_recording: bool,
}

impl Permissions {
    pub(super) fn granted(&self) -> bool {
        self.accessibility && self.screen_recording
    }
}
