import SwiftUI

@main
struct AgentApp: App {
    @State private var model = ChatModel()

    var body: some Scene {
        WindowGroup {
            ChatView(model: model)
        }
    }
}
