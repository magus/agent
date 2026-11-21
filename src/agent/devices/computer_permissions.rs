use std::io;
use std::path::Path;

#[cfg(target_os = "macos")]
pub(super) fn read(runtime_dir: &Path) -> io::Result<bool> {
    use std::process::{Command, Stdio};
    use std::time::{Duration, Instant};

    let executable = std::env::current_exe()?;
    let helper = executable
        .parent()
        .ok_or_else(|| io::Error::other("Missing executable directory"))?
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
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let output = child.wait_with_output()?;
    let permissions: serde_json::Value = serde_json::from_slice(&output.stdout)?;
    Ok(output.status.success()
        && permissions["accessibility"] == true
        && permissions["screen_recording"] == true)
}

#[cfg(not(target_os = "macos"))]
pub(super) fn read(_runtime_dir: &Path) -> io::Result<bool> {
    Ok(false)
}
