#if os(macOS)
import AppKit

@MainActor
final class TestAppDelegate: NSObject, NSApplicationDelegate {
    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool { false }
}

@MainActor
enum FindAfterClose {
    static func check() async {
        NSApp.setActivationPolicy(.regular)
        NSApp.activate(ignoringOtherApps: true)
        let startup_deadline = ContinuousClock.now + .seconds(5)
        while NSApp.keyWindow == nil && ContinuousClock.now < startup_deadline {
            try? await Task.sleep(for: .milliseconds(50))
        }
        guard let window = NSApp.keyWindow,
              let edit = NSApp.mainMenu?.item(withTitle: "Edit")?.submenu,
              let find = edit.item(withTitle: "Find…") else {
            print("Could not locate the chat window and Find command")
            exit(1)
        }
        window.performClose(nil)
        guard !window.isVisible else {
            print("The chat window did not close")
            exit(1)
        }

        await Task.yield()
        edit.performActionForItem(at: edit.index(of: find))
        let deadline = ContinuousClock.now + .seconds(5)
        while ContinuousClock.now < deadline {
            if let editor = NSApp.keyWindow?.firstResponder as? NSTextView,
               let search = editor.delegate as? NSSearchField {
                editor.insertText("reviewcheck", replacementRange: editor.selectedRange())
                if search.stringValue == "reviewcheck" {
                    print("Find reopened chat and typed into Search titles")
                    exit(0)
                }
            }
            try? await Task.sleep(for: .milliseconds(50))
        }
        let responder = String(describing: NSApp.keyWindow?.firstResponder)
        let windows = NSApp.windows.filter(\.isVisible).map(\.title)
        print("First Find focused \(responder). Visible windows: \(windows)")
        exit(1)
    }
}
#endif
