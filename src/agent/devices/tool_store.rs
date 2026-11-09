use super::device_store::save_json;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::os::unix::net::UnixStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) struct ToolStore {
    directory: PathBuf,
    runtime_dir: PathBuf,
    job_by_id: Mutex<HashMap<String, Job>>,
    cancellation_by_id: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ToolStore {
    pub fn load(runtime_dir: PathBuf) -> io::Result<Arc<Self>> {
        let directory = runtime_dir.join("requests");
        fs::create_dir_all(&directory)?;
        let mut job_by_id = HashMap::new();
        for entry in fs::read_dir(&directory)? {
            let path = entry?.path();
            if path.extension().is_none_or(|extension| extension != "json") {
                continue;
            }
            let mut job: Job = serde_json::from_slice(&fs::read(&path)?)?;
            if job.status == "running" {
                job.status = "interrupted".into();
                job.result = json!({"error": "Agent stopped before recording the result. Do not retry blindly."});
                save_json(&path, &job)?;
            }
            job_by_id.insert(job.id.clone(), job);
        }
        Ok(Arc::new(Self {
            directory,
            runtime_dir,
            job_by_id: Mutex::new(job_by_id),
            cancellation_by_id: Mutex::new(HashMap::new()),
        }))
    }

    pub fn submit(self: &Arc<Self>, owner: &str, id: &str, tool: Tool) -> io::Result<Job> {
        uuid::Uuid::parse_str(id).map_err(|_| io::Error::other("Request ID must be a UUID"))?;
        tool.validate()?;
        let mut job_by_id = self.job_by_id.lock().unwrap();
        if let Some(job) = job_by_id.get(id) {
            if job.owner != owner || job.tool != tool {
                return Err(io::Error::other(
                    "Request ID already belongs to another request",
                ));
            }
            return Ok(job.clone());
        }

        let running_count = job_by_id
            .values()
            .filter(|job| job.status == "running")
            .count();
        if running_count >= 16 {
            return Err(io::Error::other(
                "Too many running requests; wait or cancel one",
            ));
        }

        let job = Job {
            id: id.into(),
            owner: owner.into(),
            tool: tool.clone(),
            status: "running".into(),
            result: Value::Null,
        };
        save_json(&self.directory.join(format!("{id}.json")), &job)?;
        job_by_id.insert(id.into(), job.clone());
        let cancelled = Arc::new(AtomicBool::new(false));
        self.cancellation_by_id
            .lock()
            .unwrap()
            .insert(id.into(), cancelled.clone());
        let store = self.clone();
        let id = id.to_string();
        std::thread::spawn(move || {
            let result = store.execute(tool, &cancelled);
            let mut job_by_id = store.job_by_id.lock().unwrap();
            let job = job_by_id.get_mut(&id).unwrap();
            match result {
                Ok(result) => {
                    let computer_dispatched = matches!(job.tool, Tool::Computer { .. })
                        && result.get("dispatched") != Some(&Value::Bool(false));
                    let shell_failed =
                        matches!(job.tool, Tool::Shell { .. }) && result["exit_code"] != 0;
                    job.status = if cancelled.load(Ordering::Relaxed) && !computer_dispatched {
                        "cancelled"
                    } else if result.get("ok") == Some(&Value::Bool(false)) || shell_failed {
                        "failed"
                    } else {
                        "completed"
                    }
                    .into();
                    job.result = result;
                    if computer_dispatched && cancelled.load(Ordering::Relaxed) {
                        job.result["cancellation_requested"] = true.into();
                    }
                }
                Err(error) => {
                    job.status = "failed".into();
                    job.result = json!({"error": error.to_string()});
                }
            }
            if let Err(error) = save_json(&store.directory.join(format!("{id}.json")), job) {
                eprintln!("Could not save tool result: {error}");
            }
            store.cancellation_by_id.lock().unwrap().remove(&id);
        });
        Ok(job)
    }

    pub fn get(&self, owner: &str, id: &str) -> io::Result<Job> {
        self.job_by_id
            .lock()
            .unwrap()
            .get(id)
            .filter(|job| job.owner == owner)
            .cloned()
            .ok_or_else(|| io::Error::other("Request not found"))
    }

    pub fn cancel(&self, owner: &str, id: &str) -> io::Result<Job> {
        let job = self.get(owner, id)?;
        if let Some(cancelled) = self.cancellation_by_id.lock().unwrap().get(id) {
            cancelled.store(true, Ordering::Relaxed);
        }
        Ok(job)
    }

    fn execute(&self, tool: Tool, cancelled: &AtomicBool) -> io::Result<Value> {
        match tool {
            Tool::Shell {
                command,
                cwd,
                timeout_seconds,
            } => run_shell(&command, &cwd, timeout_seconds, cancelled),
            Tool::Computer { action } => {
                if cancelled.load(Ordering::Relaxed) {
                    return Ok(json!({"dispatched": false}));
                }
                let mut socket = UnixStream::connect(self.runtime_dir.join("agent.sock"))?;
                socket.set_read_timeout(Some(Duration::from_secs(40)))?;
                socket.set_write_timeout(Some(Duration::from_secs(10)))?;
                socket.write_all(&serde_json::to_vec(&action)?)?;
                socket.write_all(b"\n")?;
                let mut line = String::new();
                BufReader::new(socket.take(8 * 1024 * 1024)).read_line(&mut line)?;
                let mut result: Value = serde_json::from_str(&line)?;
                if let Some(path) = result["result"]["capture"]["path"].as_str() {
                    let path = PathBuf::from(path).canonicalize()?;
                    if !path.starts_with(self.runtime_dir.canonicalize()?) {
                        return Err(io::Error::other(
                            "Capture path is outside the runtime directory",
                        ));
                    }
                    let image = fs::read(path)?;
                    if image.len() > 32 * 1024 * 1024 {
                        return Err(io::Error::other("Capture exceeds 32 MiB"));
                    }
                    result["image_base64"] = base64::engine::general_purpose::STANDARD
                        .encode(image)
                        .into();
                }
                Ok(result)
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Job {
    pub id: String,
    owner: String,
    tool: Tool,
    pub status: String,
    pub result: Value,
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tool", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Tool {
    Shell {
        command: String,
        cwd: PathBuf,
        timeout_seconds: u64,
    },
    Computer {
        action: Value,
    },
}

impl Tool {
    pub(super) fn validate(&self) -> io::Result<()> {
        match self {
            Self::Shell {
                command,
                cwd,
                timeout_seconds,
            } => {
                if command.is_empty()
                    || command.len() > 1024 * 1024
                    || !cwd.is_absolute()
                    || !(1..=86400).contains(timeout_seconds)
                {
                    return Err(io::Error::other(
                        "Shell needs a command, absolute cwd, and timeout of 1–86400 seconds",
                    ));
                }
            }
            Self::Computer { action } => {
                if serde_json::to_vec(action)?.len() > 8 * 1024 * 1024 {
                    return Err(io::Error::other("Computer request exceeds 8 MiB"));
                }
            }
        }
        Ok(())
    }
}

fn run_shell(
    command: &str,
    cwd: &PathBuf,
    timeout_seconds: u64,
    cancelled: &AtomicBool,
) -> io::Result<Value> {
    let mut child = Command::new("/bin/zsh")
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .process_group(0)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_worker = std::thread::spawn(move || read_output(stdout));
    let stderr_worker = std::thread::spawn(move || read_output(stderr));
    let started = Instant::now();
    let mut timed_out = false;
    let exit = loop {
        if let Some(exit) = child.try_wait()? {
            break exit;
        }
        timed_out = started.elapsed() >= Duration::from_secs(timeout_seconds);
        if timed_out || cancelled.load(Ordering::Relaxed) {
            // The child owns its process group. Stop shell descendants too.
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            break child.wait()?;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    // Background children must not retain the output pipes after the shell finishes.
    unsafe {
        libc::kill(-(child.id() as i32), libc::SIGKILL);
    }
    let (stdout, stdout_truncated) = stdout_worker
        .join()
        .map_err(|_| io::Error::other("Output reader failed"))??;
    let (stderr, stderr_truncated) = stderr_worker
        .join()
        .map_err(|_| io::Error::other("Output reader failed"))??;
    Ok(
        json!({"exit_code": exit.code(), "stdout": stdout, "stderr": stderr,
        "stdout_truncated": stdout_truncated, "stderr_truncated": stderr_truncated, "timed_out": timed_out}),
    )
}

fn read_output(mut stream: impl Read) -> io::Result<(String, bool)> {
    let mut output = Vec::new();
    let mut buffer = [0; 8192];
    let mut truncated = false;
    loop {
        let count = stream.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let retained = count.min((1024 * 1024_usize).saturating_sub(output.len()));
        output.extend_from_slice(&buffer[..retained]);
        truncated |= retained < count;
    }
    Ok((String::from_utf8_lossy(&output).into_owned(), truncated))
}
