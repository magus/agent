use std::io;
use std::path::Path;

/// Checks desktop permissions through a fresh helper process.
/// The error explains what is missing and how the user can fix it, for both the model and the UI.
pub(super) fn check(runtime_dir: &Path) -> Result<(), String> {
    match read(runtime_dir) {
        Ok((accessibility, screen_recording)) => status(accessibility, screen_recording),
        Err(error) => Err(format!(
            "Computer use is unavailable because Agent could not check desktop permissions ({error}). \
             Make sure Agent.app is installed and open on this Mac."
        )),
    }
}

fn status(accessibility: bool, screen_recording: bool) -> Result<(), String> {
    let mut missing = Vec::new();
    if !accessibility {
        missing.push("Accessibility");
    }
    if !screen_recording {
        missing.push("Screen & System Audio Recording");
    }
    if missing.is_empty() {
        return Ok(());
    }
    let noun = if missing.len() > 1 {
        "permissions"
    } else {
        "permission"
    };
    let missing = missing.join(" and ");
    Err(format!(
        "Computer use is off because Agent lacks the {missing} {noun} on this Mac. \
         Open Agent → Desktop permissions… (or System Settings → Privacy & Security → {missing}), \
         turn on Agent, then quit and reopen Agent."
    ))
}

#[cfg(target_os = "macos")]
fn read(runtime_dir: &Path) -> io::Result<(bool, bool)> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let executable = std::env::current_exe()?;
    let helper = executable
        .parent()
        .ok_or_else(|| io::Error::other("missing executable directory"))?
        .join("desktop_service");
    let mut child = Command::new(helper)
        .arg(runtime_dir)
        .arg("--permissions")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let started = Instant::now();
    while child.try_wait()?.is_none() {
        if started.elapsed() > Duration::from_secs(2) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(io::Error::other("the permission check timed out"));
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(io::Error::other("the permission check failed"));
    }
    let permissions: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok((
        permissions["accessibility"] == true,
        permissions["screen_recording"] == true,
    ))
}

#[cfg(not(target_os = "macos"))]
fn read(_runtime_dir: &Path) -> io::Result<(bool, bool)> {
    Err(io::Error::other("desktop control requires macOS"))
}

#[cfg(test)]
mod tests {
    use super::status;

    #[test]
    fn reasons_name_each_missing_permission_and_the_fix() {
        assert_eq!(status(true, true), Ok(()));
        let accessibility = status(false, true).unwrap_err();
        assert!(accessibility.contains("lacks the Accessibility permission"));
        assert!(accessibility.contains("Privacy & Security → Accessibility"));
        let both = status(false, false).unwrap_err();
        assert!(both.contains("Accessibility and Screen & System Audio Recording permissions"));
        assert!(both.contains("quit and reopen Agent"));
    }
}
