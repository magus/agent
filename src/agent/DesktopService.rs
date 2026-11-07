use serde::Deserialize;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

pub(crate) struct DesktopService {
    helper: PathBuf,
    pub(crate) runtime_dir: PathBuf,
    child: Mutex<Option<Child>>,
}

impl DesktopService {
    pub(crate) fn new() -> io::Result<Self> {
        let executable = std::env::current_exe()?;
        let helper = executable.parent().unwrap().join("desktop_service");
        let home = std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is missing"))?;
        let runtime_dir = PathBuf::from(home).join("Library/Application Support/Agent");
        fs::create_dir_all(&runtime_dir)?;
        fs::set_permissions(&runtime_dir, fs::Permissions::from_mode(0o700))?;

        Ok(Self {
            helper,
            runtime_dir,
            child: Mutex::new(None),
        })
    }

    pub(crate) fn start(&self) -> io::Result<()> {
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

    pub(crate) fn permissions(&self) -> io::Result<Permissions> {
        serde_json::from_value(self.call("--permissions")?).map_err(io::Error::other)
    }

    pub(crate) fn call(&self, mode: &str) -> io::Result<Value> {
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

    pub(crate) fn stop(&self) {
        if let Some(mut child) = self.child.lock().unwrap().take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }

    pub(crate) fn stopped(&self) -> io::Result<bool> {
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
pub(crate) struct Permissions {
    pub(crate) accessibility: bool,
    pub(crate) screen_recording: bool,
}

impl Permissions {
    pub(crate) fn granted(&self) -> bool {
        self.accessibility && self.screen_recording
    }
}
