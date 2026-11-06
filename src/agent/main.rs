use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

fn main() {
    if let Err(error) = run() {
        eprintln!("Agent: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let argument_list: Vec<String> = std::env::args().skip(1).collect();
    let mode = match argument_list.as_slice() {
        [] => "--serve",
        [argument] if argument == "--permissions" => "--permissions",
        [argument] if argument == "--help" || argument == "-h" => {
            println!("Usage: Agent [--permissions]");
            return Ok(());
        }
        _ => return Err(io::Error::other("Unsupported arguments; use --help")),
    };

    let executable = std::env::current_exe()?;
    let helper = executable.parent().unwrap().join("desktop_service");
    let home = std::env::var_os("HOME").ok_or_else(|| io::Error::other("HOME is missing"))?;
    let runtime = std::path::PathBuf::from(home).join("Library/Application Support/Agent");
    fs::create_dir_all(&runtime)?;
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700))?;

    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(runtime.join("service.log"))?;

    // Signal handlers only set an atomic flag; the main loop reaps the helper.
    unsafe {
        signal(2, stop as *const () as usize);
        signal(15, stop as *const () as usize);
    }

    let mut child = Command::new(helper)
        .arg(&runtime)
        .arg(mode)
        .stdin(Stdio::null())
        .stdout(log.try_clone()?)
        .stderr(log)
        .spawn()?;
    loop {
        if let Some(status) = child.try_wait()? {
            return if status.success() {
                Ok(())
            } else {
                Err(io::Error::other(status.to_string()))
            };
        }

        if STOP_REQUESTED.load(Ordering::Relaxed) || runtime.join("stop").exists() {
            child.kill()?;
            child.wait()?;
            return Ok(());
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}

extern "C" fn stop(_: i32) {
    STOP_REQUESTED.store(true, Ordering::Relaxed);
}

unsafe extern "C" {
    fn signal(number: i32, handler: usize) -> usize;
}
