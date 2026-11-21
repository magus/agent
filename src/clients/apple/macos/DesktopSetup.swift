#if os(macOS)
import Foundation

actor DesktopSetup {
    func permissions() throws -> DesktopPermissions {
        let response = try call("--permissions")
        return DesktopPermissions(
            accessibility: response["accessibility"] as? Bool == true,
            screen_recording: response["screen_recording"] as? Bool == true
        )
    }

    func open_accessibility() throws { _ = try call("--open-accessibility") }
    func open_screen_recording() throws { _ = try call("--open-screen-recording") }

    func capture() throws -> String {
        let response = try call("--setup-capture")
        guard let capture = response["capture"] as? [String: Any], let path = capture["path"] as? String else {
            throw ClientError(message: "No screenshot returned.")
        }
        return path
    }

    private func call(_ mode: String) throws -> [String: Any] {
        let helper = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/desktop_service")
        let directory = try FileManager.default.url(
            for: .applicationSupportDirectory, in: .userDomainMask,
            appropriateFor: nil, create: true
        ).appendingPathComponent("Agent", isDirectory: true)
        let output = Pipe()
        let process = Process()
        process.executableURL = helper
        process.arguments = [directory.path, mode]
        process.standardInput = FileHandle.nullDevice
        process.standardOutput = output
        process.standardError = FileHandle.nullDevice
        try process.run()

        let data = output.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        guard let response = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw ClientError(message: "Desktop service returned an invalid response.")
        }
        guard process.terminationStatus == 0, response["ok"] as? Bool != false else {
            throw ClientError(message: response["error"] as? String ?? "Desktop service failed.")
        }
        return response
    }
}
#endif
