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
        .commands { AgentCommands() }

        Window("Desktop permissions", id: "desktop-permissions") {
            DesktopPermissionsView()
        }
        .defaultSize(width: 480, height: 440)
    }
}
