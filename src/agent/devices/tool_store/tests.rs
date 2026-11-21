use super::*;
use std::os::unix::net::UnixListener;

#[test]
fn unavailable_permission_probe_blocks_dispatch_despite_cached_grants() {
    let executable = std::env::current_exe().unwrap();
    assert!(
        !executable
            .parent()
            .unwrap()
            .join("desktop_service")
            .exists()
    );

    let directory =
        PathBuf::from(".scratch/2026-11-11-12-42-native-ui").join(uuid::Uuid::new_v4().to_string());
    let store = ToolStore::load(directory.clone()).unwrap();
    *store.computer_permissions.lock().unwrap() = Some((Instant::now(), true));

    let listener = UnixListener::bind(directory.join("agent.sock")).unwrap();
    listener.set_nonblocking(true).unwrap();

    let result = store.execute(
        Tool::Computer {
            action: json!({"operation": "capture"}),
        },
        &AtomicBool::new(false),
    );

    assert_eq!(
        result.unwrap_err().to_string(),
        "Desktop permissions are not enabled."
    );
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );

    drop(listener);
    fs::remove_dir_all(directory).unwrap();
}
