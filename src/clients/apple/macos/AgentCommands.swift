#if os(macOS)
import SwiftUI

struct AgentCommands: Commands {
    @Environment(\.openWindow) private var open_window

    var body: some Commands {
        CommandGroup(after: .appSettings) {
            Button("Desktop permissions…") { open_window(id: "desktop-permissions") }
        }
    }
}
#endif
