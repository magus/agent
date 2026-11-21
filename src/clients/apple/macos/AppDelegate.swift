#if os(macOS)
import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    private let desktop_service = DesktopService()

    func applicationDidFinishLaunching(_ notification: Notification) {
        NSApp.setActivationPolicy(.regular)
        do { try desktop_service.start() }
        catch { NSLog("Desktop service could not start: %@", error.localizedDescription) }
    }

    func applicationWillTerminate(_ notification: Notification) {
        desktop_service.stop()
    }

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        false
    }
}
#endif
