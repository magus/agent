#if os(macOS)
import Darwin
import Foundation

@MainActor
final class DesktopService {
    private var process: Process?
    private var desired_running = false
    private var restart_task: Task<Void, Never>?
    private var started_at = Date.distantPast
    private var restart_delay: Duration = .seconds(2)

    func start() throws {
        desired_running = true
        guard process == nil else { return }
        do { try launch() }
        catch {
            schedule_restart()
            throw error
        }
    }

    private func launch() throws {
        let helper = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/desktop_service")
        stop_orphaned_helpers(at: helper.path)
        let directory = try FileManager.default.url(
            for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true
        ).appendingPathComponent("Agent", isDirectory: true)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        let log_url = directory.appendingPathComponent("service.log")
        if !FileManager.default.fileExists(atPath: log_url.path) {
            FileManager.default.createFile(atPath: log_url.path, contents: nil, attributes: [.posixPermissions: 0o600])
        }
        let log = try FileHandle(forWritingTo: log_url)
        defer { try? log.close() }
        try log.seekToEnd()

        let process = Process()
        process.executableURL = helper
        process.arguments = [directory.path, "--serve"]
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = log
        process.standardError = log
        process.terminationHandler = { [weak self] child in
            Task { @MainActor [weak self] in
                guard let self, self.process?.processIdentifier == child.processIdentifier else { return }
                self.process = nil
                self.schedule_restart()
            }
        }
        try process.run()
        self.process = process
        started_at = Date()
    }

    // Retry quickly after a helper that ran for a while; back off while it keeps failing at once,
    // for example when another copy of Agent already serves the desktop.
    private func schedule_restart() {
        guard desired_running else { return }
        if Date().timeIntervalSince(started_at) > 30 { restart_delay = .seconds(2) }
        let delay = restart_delay
        restart_delay = min(restart_delay * 2, .seconds(60))
        restart_task?.cancel()
        restart_task = Task { [weak self] in
            do { try await Task.sleep(for: delay) }
            catch { return }
            guard let self, self.desired_running else { return }
            do { try self.start() }
            catch { NSLog("Desktop service restart failed: %@", error.localizedDescription) }
        }
    }

    func stop() {
        desired_running = false
        restart_task?.cancel()
        guard let process else { return }
        process.terminationHandler = nil
        if process.isRunning {
            process.terminate()
            process.waitUntilExit()
        }
        self.process = nil
    }
}

/// Stops helpers from this app bundle whose app is gone (re-parented to launchd). They keep the
/// desktop socket and block the helper this launch starts. Helpers that still have a parent belong
/// to another running copy of Agent and are left alone.
private func stop_orphaned_helpers(at path: String) {
    let count = proc_listallpids(nil, 0)
    guard count > 0 else { return }
    var pid_list = [pid_t](repeating: 0, count: Int(count) + 32)
    let filled = proc_listallpids(&pid_list, Int32(pid_list.count * MemoryLayout<pid_t>.size))
    guard filled > 0 else { return }
    var path_buffer = [CChar](repeating: 0, count: Int(MAXPATHLEN))
    for pid in pid_list.prefix(Int(filled)) where pid > 1 && pid != getpid() {
        var info = proc_bsdinfo()
        let size = Int32(MemoryLayout<proc_bsdinfo>.size)
        guard proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, size) == size,
              info.pbi_uid == getuid(), info.pbi_ppid == 1,
              proc_pidpath(pid, &path_buffer, UInt32(path_buffer.count)) > 0,
              String(cString: path_buffer) == path else { continue }
        NSLog("Stopping orphaned desktop service %d", pid)
        kill(pid, SIGTERM)
    }
}
#endif
