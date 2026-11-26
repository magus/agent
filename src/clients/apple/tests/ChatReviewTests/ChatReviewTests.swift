import SwiftUI
import WebKit
#if os(macOS)
import ApplicationServices
#endif

// Runs the production views with a local bridge fixture and no paired devices.
@main
struct ChatReviewTests: App {
    @State private var model = ChatModel()
    #if os(macOS)
    @NSApplicationDelegateAdaptor(TestAppDelegate.self) private var delegate
    @State private var checking_find = false
    #endif

    init() {
        #if os(macOS)
        DesktopRestartDelayTests.check()
        #endif
    }

    var body: some Scene {
        #if os(macOS)
        Window("Chat", id: "chat") { content }
            .commands { AgentCommands(model: model) }
        #else
        WindowGroup { content }
        #endif
    }

    @ViewBuilder private var content: some View {
        #if os(macOS)
        if CommandLine.arguments.contains("--find") {
            ChatView(model: model)
                .onAppear {
                    guard !checking_find else { return }
                    checking_find = true
                    // Closing the window must not cancel the test.
                    Task { await FindAfterClose.check() }
                }
        } else { fixture_view }
        #else
        fixture_view
        #endif
    }

    @ViewBuilder private var fixture_view: some View {
        if CommandLine.arguments.contains("--startup-error") {
            ChatView(model: model)
                #if os(macOS)
                .task { await check_error_view(message: "Startup fixture failed") }
                #endif
        } else if CommandLine.arguments.contains("--connection-error") {
            ServiceConnectionView(model: model)
                #if os(macOS)
                .task { await check_error_view(message: "Service fixture is offline", button: "Offline service") }
                #endif
                .onAppear { model.service_list = [Device(id: "offline", name: "Offline service")] }
        } else {
            ConversationJSONView(service: "fixture", id: "long-line")
                .task { await check_renderer() }
        }
    }

    #if os(macOS)
    @MainActor
    private func check_error_view(message: String, button: String? = nil) async {
        var waiting_for_button = button
        let application = AXUIElementCreateApplication(getpid())
        AXUIElementSetAttributeValue(application, "AXEnhancedUserInterface" as CFString, kCFBooleanTrue)
        let deadline = ContinuousClock.now + .seconds(10)
        while ContinuousClock.now < deadline {
            var stack = attribute(application, kAXWindowsAttribute) as? [AXUIElement] ?? []
            while let element = stack.popLast() {
                let title = attribute(element, kAXTitleAttribute) as? String
                let label = attribute(element, kAXDescriptionAttribute) as? String
                if let button = waiting_for_button, title == button || label == button {
                    if AXUIElementPerformAction(element, kAXPressAction as CFString) == .success {
                        waiting_for_button = nil
                    }
                }
                let value = attribute(element, kAXValueAttribute) as? String
                if [title, label, value].contains(message) {
                    print("Visible error passed: \(message)")
                    exit(0)
                }
                let children = attribute(element, kAXChildrenAttribute) as? [AXUIElement] ?? []
                stack.append(contentsOf: children)
            }
            try? await Task.sleep(for: .milliseconds(50))
        }
        print("Missing visible error: \(message)")
        exit(1)
    }

    private func attribute(_ element: AXUIElement, _ name: String) -> CFTypeRef? {
        var value: CFTypeRef?
        AXUIElementCopyAttributeValue(element, name as CFString, &value)
        return value
    }
    #endif

    @MainActor
    private func check_renderer() async {
        do {
            let deadline = ContinuousClock.now + .seconds(20)
            while ContinuousClock.now < deadline {
                if let web_view = find_web_view(),
                   let ready = try await web_view.evaluateJavaScript("document.querySelector('code')?.textContent.includes('END_MARKER') ?? false") as? Bool,
                   ready {
                    let file = Bundle.main.url(forResource: "checkLayout", withExtension: "js")!
                    let script = try String(contentsOf: file, encoding: .utf8)
                    let result = try await web_view.callAsyncJavaScript(script, arguments: [:], in: nil, contentWorld: .page)
                    print("Layout passed: \(result ?? "")")
                    exit(0)
                }
                try await Task.sleep(for: .milliseconds(50))
            }
            throw ClientError(message: "Timed out waiting for the JSON viewer")
        } catch {
            print("Layout failed: \(error)")
            exit(1)
        }
    }

    @MainActor
    private func find_web_view() -> WKWebView? {
        #if os(macOS)
        var views = NSApp.windows.compactMap(\.contentView)
        #else
        let scenes = UIApplication.shared.connectedScenes.compactMap { $0 as? UIWindowScene }
        var views: [UIView] = scenes.flatMap(\.windows)
        #endif
        while let view = views.popLast() {
            if let web_view = view as? WKWebView { return web_view }
            views.append(contentsOf: view.subviews)
        }
        return nil
    }
}

// Only the bridge responses used by these three views are needed.
@_cdecl("agent_request")
func fixture_request(_ input: UnsafePointer<CChar>) -> UnsafeMutablePointer<CChar>? {
    let data = Data(String(cString: input).utf8)
    let request = try! JSONSerialization.jsonObject(with: data) as! [String: Any]
    let response: [String: Any]
    switch request["action"] as? String {
    case "conversation_json":
        let line = String(repeating: "long JSON string ", count: 60) + "END_MARKER"
        response = ["ok": true, "result": "\"\(line)\""]
    case "initialize":
        response = ["ok": false, "error": "Startup fixture failed"]
    case "connect":
        response = ["ok": false, "error": "Service fixture is offline"]
    default:
        preconditionFailure("Unexpected fixture request")
    }
    let json = try! JSONSerialization.data(withJSONObject: response)
    return strdup(String(decoding: json, as: UTF8.self))
}

@_cdecl("agent_string_free")
func fixture_free(_ value: UnsafeMutablePointer<CChar>?) {
    free(value)
}
