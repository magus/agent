import AppKit
import ApplicationServices
import Foundation
import Darwin

@main
struct service_main {
    static func main() {
        umask(0o077)
        let argument_list = Array(CommandLine.arguments.dropFirst())
        guard argument_list.count >= 2 else {
            fputs("Usage: desktop_service OUTPUT_DIR --serve | --send REQUEST.json | --stdio | --job JOB.json | --permissions\n", stderr)
            exit(2)
        }
        let output_dir = URL(fileURLWithPath: argument_list[0], isDirectory: true)

        do {
            let socket_path = output_dir.appendingPathComponent("agent.sock").path
            if argument_list[1] == "--send" {
                guard argument_list.count == 3 else { throw control_error("Missing request file") }
                let request = try Data(contentsOf: URL(fileURLWithPath: argument_list[2]))
                let response = try local_socket.send(socket_path, request: request)
                FileHandle.standardOutput.write(response + Data([10]))
                let result = try JSONSerialization.jsonObject(with: response) as? [String: Any]
                if result?["ok"] as? Bool != true { exit(1) }
                return
            }
            _ = NSApplication.shared
            if argument_list[1] == "--permissions" {
                emit(["accessibility": AXIsProcessTrusted(),
                      "screen_recording": CGPreflightScreenCaptureAccess(),
                      "pid": getpid()])
                return
            }

            let controller = desktop_control()
            try FileManager.default.createDirectory(at: output_dir, withIntermediateDirectories: true,
                                                     attributes: [.posixPermissions: 0o700])
            try FileManager.default.setAttributes([.posixPermissions: 0o700], ofItemAtPath: output_dir.path)
            switch argument_list[1] {
            case "--serve":
                try local_socket.serve(socket_path) { request in
                    let result = handle(request, controller: controller, output_dir: output_dir)
                    return try! JSONSerialization.data(withJSONObject: result, options: [.sortedKeys])
                }
            case "--stdio":
                while let line = readLine() {
                    guard let data = line.data(using: .utf8) else { continue }
                    let response = handle(data, controller: controller, output_dir: output_dir)
                    emit(response)
                }
            case "--job":
                guard argument_list.count == 3 else { throw control_error("Missing job file") }
                let data = try Data(contentsOf: URL(fileURLWithPath: argument_list[2]))
                guard let job = try JSONSerialization.jsonObject(with: data) as? [String: Any],
                      let action_list = job["actions"] as? [[String: Any]] else {
                    throw control_error("Job must contain an actions array")
                }
                let delay = job["start_delay_seconds"] as? Double ?? 0
                guard delay >= 0, delay <= 3600 else { throw control_error("Invalid start delay") }
                Thread.sleep(forTimeInterval: delay)
                for action in action_list {
                    let request = try JSONSerialization.data(withJSONObject: action)
                    let response = handle(request, controller: controller, output_dir: output_dir)
                    emit(response)
                    guard response["ok"] as? Bool == true else { exit(1) }
                }
            default:
                throw control_error("Unknown mode")
            }
        } catch {
            emit(["ok": false, "error": String(describing: error)])
            exit(1)
        }
    }

    private static func handle(
        _ data: Data,
        controller: desktop_control,
        output_dir: URL
    ) -> [String: Any] {
        let timestamp = Date().timeIntervalSince1970
        var request_id: String?
        do {
            guard data.count <= 8 * 1024 * 1024,
                  let request = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                throw control_error("Invalid JSON request or request exceeds 8 MiB")
            }
            request_id = request["id"] as? String
            let result = try controller.execute(request, output_dir: output_dir)
            return ["ok": true, "id": request_id ?? NSNull(), "started_at": timestamp,
                    "finished_at": Date().timeIntervalSince1970, "result": result]
        } catch {
            // A failed input request may have acted before failing. Never replay it automatically.
            return ["ok": false, "id": request_id ?? NSNull(), "started_at": timestamp,
                    "error": String(describing: error), "automatic_retry": false]
        }
    }

    private static func emit(_ value: [String: Any]) {
        do {
            let data = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data([10]))
        } catch {
            fputs("Cannot encode response\n", stderr)
            exit(1)
        }
    }
}
