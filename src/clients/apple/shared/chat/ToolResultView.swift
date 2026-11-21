import SwiftUI

struct ToolResultView: View {
    let entry: Entry
    let request: ToolRequest
    let device_name: String
    @Binding var expanded: Bool

    var body: some View {
        DisclosureGroup(isExpanded: $expanded) {
            VStack(alignment: .leading, spacing: 12) {
                Text(request.tool.formatted)
                    .font(.system(.caption, design: .monospaced))
                    .textSelection(.enabled)

                if !entry.text.isEmpty {
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
    }

    private var summary: String {
        if let command = request.tool["command"]?.string {
            return command.split(whereSeparator: \.isWhitespace).joined(separator: " ")
        }
        return request.tool["action"]?["operation"]?.string?.capitalized ?? "Computer action"
    }
}
