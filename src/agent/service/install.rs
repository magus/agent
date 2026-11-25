use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::process::{Command, Stdio};

pub(crate) fn install() -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let home = std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is missing"))?;
    let directory = std::path::PathBuf::from(home).join("Library/LaunchAgents");
    fs::create_dir_all(&directory)?;
    let path = directory.join("com.iamnoah.agent.service.plist");
    let runtime = crate::platform::runtime_dir()?.join("service");
    fs::create_dir_all(&runtime)?;

    let configuration = json!({
        "Label":"com.iamnoah.agent.service",
        "ProgramArguments":[executable, "--service"],
        "RunAtLoad":true,
        "KeepAlive":true,
        "ThrottleInterval":10,
        "StandardOutPath":runtime.join("service.log"),
        "StandardErrorPath":runtime.join("service.log")
    });
    let mut converter = Command::new("/usr/bin/plutil")
        .args(["-convert", "xml1", "-o"])
        .arg(&path)
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()?;
    converter
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&configuration)?)?;
    if !converter.wait()?.success() {
        return Err(io::Error::other(
            "Could not write service startup configuration.",
        ));
    }

    let domain = format!("gui/{}", unsafe { libc::getuid() });
    let label = format!("{domain}/com.iamnoah.agent.service");
    let _ = Command::new("/bin/launchctl")
        .args(["bootout", &label])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    // bootout returns before launchd finishes removing the old job; bootstrapping during that
    // window fails with "Bootstrap failed: 5: Input/output error". Retry briefly instead of
    // leaving the service stopped.
    let mut started = false;
    for _ in 0..10 {
        let status = Command::new("/bin/launchctl")
            .args(["bootstrap", &domain])
            .arg(&path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            started = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    if !started {
        return Err(io::Error::other(
            "Could not start Agent's central service. Run `launchctl bootstrap gui/$(id -u) \
             ~/Library/LaunchAgents/com.iamnoah.agent.service.plist` to retry.",
        ));
    }
    println!(
        "Central service installed. It starts at login and continues while the screen is locked."
    );
    Ok(())
}
