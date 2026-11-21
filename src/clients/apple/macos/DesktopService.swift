#if os(macOS)
import Foundation

@MainActor
final class DesktopService {
    private var process: Process?
    private var desired_running = false
    private var restart_task: Task<Void, Never>?

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
    }

    private func schedule_restart() {
        guard desired_running else { return }
        restart_task?.cancel()
        restart_task = Task { [weak self] in
            do { try await Task.sleep(for: .seconds(2)) }
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
#endif
