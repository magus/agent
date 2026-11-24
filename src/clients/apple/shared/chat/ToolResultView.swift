import SwiftUI

struct ToolResultView: View {
    let entry: Entry
    let request: ToolRequest
    let device_name: String
    @Binding var expanded: Bool

    var body: some View {
        DisclosureGroup(isExpanded: $expanded) {
            VStack(alignment: .leading, spacing: 12) {
                if let command = request.tool["command"]?.string {
                    // The result text starts with the full command; until it arrives, show the command.
                    if entry.text.isEmpty {
                        Text(command)
                            .font(.system(.callout, design: .monospaced))
                            .textSelection(.enabled)
                    }
                    if let directory = request.tool["cwd"]?.string, !directory.isEmpty {
                        Text("Ran in \(directory)").font(.caption).textSelection(.enabled)
                    }
                } else {
                    Text(request.tool.formatted)
                        .font(.system(.caption, design: .monospaced))
                        .textSelection(.enabled)
                }

                if is_computer {
                    if let outcome = computer_outcome {
                        Text(outcome).font(.callout).textSelection(.enabled)
                    }
                } else if !entry.text.isEmpty {
                    Text(entry.text)
                        .font(.system(.callout, design: .monospaced))
                        .textSelection(.enabled)
                }

                if let path = entry.image { ScreenshotView(path: path) }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.vertical, 8)
        } label: {
            HStack(spacing: 8) {
                Image(systemName: request.tool["tool"]?.string == "computer" ? "desktopcomputer" : "terminal")
                Text(summary).lineLimit(1).truncationMode(.tail)
                Spacer(minLength: 8)
                Text(device_name).font(.caption).lineLimit(1)
                if !request.finished {
                    ProgressView().controlSize(.mini)
                } else {
                    Image(systemName: request.status == "completed" ? "checkmark" : "exclamationmark.circle")
                        .accessibilityLabel(request.status)
                }
            }
            .font(.callout)
            .foregroundStyle(.secondary)
        }
        .tint(.secondary)
    }

    private var is_computer: Bool { request.tool["tool"]?.string == "computer" }

    /// One readable line for a finished computer action. The full result stays in the conversation JSON.
    private var computer_outcome: String? {
        guard request.finished, let data = entry.text.data(using: .utf8),
              let job = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else { return nil }
        let result = job["result"] as? [String: Any] ?? [:]
        if let error = result["error"] as? String { return error }
        let action = result["result"] as? [String: Any] ?? [:]
        var parts: [String] = []
        if let capture = action["capture"] as? [String: Any],
           let width = capture["width"] as? Int, let height = capture["height"] as? Int {
            parts.append("Screenshot \(width) × \(height)")
        }
        if let locked = action["locked"] as? Bool {
            parts.append(locked ? "Mac was locked" : "Mac was unlocked")
        }
        return parts.isEmpty ? nil : parts.joined(separator: " · ")
    }

    private var summary: String {
        if let command = request.tool["command"]?.string {
            return command.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        }
        return request.tool["action"]?["operation"]?.string?.capitalized ?? "Computer action"
    }
}
