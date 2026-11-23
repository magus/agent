#if os(macOS)
import SwiftUI

struct AgentCommands: Commands {
    let model: ChatModel
    @Environment(\.openWindow) private var open_window

    var body: some Commands {
        // The chat window shares one model, so a second window would only mirror the first.
        CommandGroup(replacing: .newItem) {
            Button("New Conversation") { Task { await model.create() } }
                .keyboardShortcut("n")
                .disabled(!model.initialized || model.service_host.isEmpty || model.creating)
        }
        CommandGroup(replacing: .appSettings) {
            Button("Conversation Settings…") { model.sheet = .settings }
                .keyboardShortcut(",")
                .disabled(model.service_host.isEmpty)
            Button("Devices…") { model.sheet = .devices }
                .disabled(!model.initialized)
            Button("ChatGPT Account…") { model.sheet = .chatgpt }
                .disabled(model.service_host.isEmpty)
            Button("Connect to Service…") { model.sheet = .connection }
                .disabled(!model.initialized)
            Divider()
            Button("Desktop Permissions…") { open_window(id: "desktop-permissions") }
        }
    }
}
#endif
