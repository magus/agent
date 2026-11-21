import Foundation
import SwiftUI
import Darwin

@main
struct NativeMain {
    static func main() {
        let argument_list = Array(CommandLine.arguments.dropFirst())
        let command = argument_list.first
        let service_command_list = ["--service", "--service-install", "--service-device", "--device", "--help", "-h"]
        if let command, service_command_list.contains(command) {
            run_command(argument_list)
        }
        if !argument_list.isEmpty && (argument_list.count != 1 || !["--setup", "--devices", "--chatgpt"].contains(command!)) {
            fputs("Unsupported arguments; use --help\n", stderr)
            exit(2)
        }
        AgentApp.main()
    }

    private static func run_command(_ argument_list: [String]) -> Never {
        let executable = Bundle.main.bundleURL.appendingPathComponent("Contents/MacOS/agent_service")
        var pointer_list = ([executable.path] + argument_list).map { strdup($0) }
        pointer_list.append(nil)
        pointer_list.withUnsafeMutableBufferPointer { buffer in
            _ = execv(executable.path, buffer.baseAddress!)
        }
        fputs("Could not start Agent command: \(String(cString: strerror(errno)))\n", stderr)
        exit(1)
    }
}
