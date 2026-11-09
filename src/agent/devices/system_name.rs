use std::io;
use std::process::Command;

pub(super) fn system_name() -> io::Result<String> {
    let output = Command::new("/usr/sbin/scutil")
        .args(["--get", "ComputerName"])
        .output()?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || name.is_empty() {
        return Err(io::Error::other("Could not read the macOS computer name"));
    }
    Ok(name)
}
