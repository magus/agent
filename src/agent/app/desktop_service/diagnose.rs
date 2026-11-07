use super::service::DesktopService;
use std::fs::OpenOptions;
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;

pub(crate) fn diagnose() -> io::Result<()> {
    let service = DesktopService::new()?;
    let response = service.call("--permissions")?;
    let mut log = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(service.runtime_dir.join("service.log"))?;
    writeln!(log, "{response}")?;
    Ok(())
}
