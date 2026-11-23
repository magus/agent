import SwiftUI

struct AgentApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var delegate
    @State private var model = ChatModel()

    var body: some Scene {
        WindowGroup {
            ChatView(model: model)
                .frame(minWidth: 720, minHeight: 500)
        }
        .windowStyle(.hiddenTitleBar)
        .defaultSize(width: 960, height: 720)
        .commands { AgentCommands(model: model) }

        WindowGroup("Conversation JSON", id: "conversation-json", for: ConversationJSONTarget.self) { $target in
            if let target {
                ConversationJSONView(service: target.service, id: target.id)
            }
        }
        .defaultSize(width: 1280, height: 900)
        .commandsRemoved()

        Window("Desktop Permissions", id: "desktop-permissions") {
            DesktopPermissionsView()
        }
        .defaultSize(width: 480, height: 440)
    }
}
