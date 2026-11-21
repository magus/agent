import SwiftUI
import WebKit

struct MarkdownContent: View, Equatable {
    let source: String
    var active = false
    @State private var content_height: CGFloat = 24
    @State private var visible = true
    @State private var measured = false
    @Environment(\.accessibilityVoiceOverEnabled) private var voice_over
    @Environment(\.colorScheme) private var color_scheme
    #if os(macOS)
    @ScaledMetric(relativeTo: .body) private var font_size = 13.0
    #else
    @ScaledMetric(relativeTo: .body) private var font_size = 17.0
    #endif

    static func == (lhs: Self, rhs: Self) -> Bool {
        lhs.source == rhs.source && lhs.active == rhs.active
    }

    var body: some View {
        WordsWebView(
            source: source,
            active: active,
            font_size: font_size,
            dark: color_scheme == .dark,
            visible: visible || !measured || voice_over,
            height: $content_height,
            measured: $measured
        )
        .frame(height: content_height)
        .onScrollVisibilityChange(threshold: 0.000_001) { visible = $0 }
        .accessibilityElement(children: .contain)
    }
}

private struct WordsWebView {
    let source: String
    let active: Bool
    let font_size: Double
    let dark: Bool
    let visible: Bool
    @Binding var height: CGFloat
    @Binding var measured: Bool

    func makeCoordinator() -> WordsCoordinator {
        WordsCoordinator(height: $height, measured: $measured)
    }

    func make_web_view(coordinator: WordsCoordinator) -> WKWebView {
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.userContentController.add(coordinator, name: "height")
        #if os(macOS)
        let view = MarkdownWebView(frame: .zero, configuration: configuration)
        view.setValue(false, forKey: "drawsBackground")
        #else
        let view = WKWebView(frame: .zero, configuration: configuration)
        #endif
        view.underPageBackgroundColor = .clear
        view.navigationDelegate = coordinator
        coordinator.view = view
        #if os(iOS)
        view.isOpaque = false
        view.backgroundColor = .clear
        view.scrollView.isScrollEnabled = false
        #endif
        if let page = Bundle.main.url(forResource: "index", withExtension: "html", subdirectory: "markdown") {
            view.loadFileURL(page, allowingReadAccessTo: page.deletingLastPathComponent())
        }
        return view
    }

    func update(host: MarkdownHostView, coordinator: WordsCoordinator) {
        if visible && coordinator.view == nil {
            coordinator.ready_for_new_view()
            host.show(make_web_view(coordinator: coordinator))
        } else if !visible {
            coordinator.view = nil
            host.clear()
        }
        coordinator.update(source: source, active: active, font_size: font_size, dark: dark)
    }
}

#if os(macOS)
extension WordsWebView: NSViewRepresentable {
    func makeNSView(context: Context) -> MarkdownHostView {
        MarkdownHostView()
    }

    func updateNSView(_ view: MarkdownHostView, context: Context) {
        update(host: view, coordinator: context.coordinator)
    }
}
#else
extension WordsWebView: UIViewRepresentable {
    func makeUIView(context: Context) -> MarkdownHostView {
        MarkdownHostView()
    }

    func updateUIView(_ view: MarkdownHostView, context: Context) {
        update(host: view, coordinator: context.coordinator)
    }
}
#endif

#if os(macOS)
private typealias MarkdownHostBase = NSView
#else
private typealias MarkdownHostBase = UIView
#endif

private final class MarkdownHostView: MarkdownHostBase {
    private var web_view: WKWebView?

    func show(_ view: WKWebView) {
        clear()
        web_view = view
        view.translatesAutoresizingMaskIntoConstraints = false
        addSubview(view)
        NSLayoutConstraint.activate([
            view.leadingAnchor.constraint(equalTo: leadingAnchor),
            view.trailingAnchor.constraint(equalTo: trailingAnchor),
            view.topAnchor.constraint(equalTo: topAnchor),
            view.bottomAnchor.constraint(equalTo: bottomAnchor),
        ])
    }

    func clear() {
        web_view?.stopLoading()
        web_view?.removeFromSuperview()
        web_view = nil
    }
}

private final class WordsCoordinator: NSObject, WKScriptMessageHandler, WKNavigationDelegate {
    weak var view: WKWebView?
    private var height: Binding<CGFloat>
    private var measured: Binding<Bool>
    private var ready = false
    private var update: [String: Any] = [:]
    private var source: String?
    private var active = false
    private var font_size = 0.0
    private var dark = false

    init(height: Binding<CGFloat>, measured: Binding<Bool>) {
        self.height = height
        self.measured = measured
    }

    func ready_for_new_view() {
        ready = false
    }

    func update(source: String, active: Bool, font_size: Double, dark: Bool) {
        guard self.source != source || self.active != active || self.font_size != font_size || self.dark != dark else {
            return
        }
        self.source = source
        self.active = active
        self.font_size = font_size
        self.dark = dark
        update = ["source": source, "active": active, "font_size": font_size, "dark": dark]
        render()
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation!) {
        guard webView === view else { return }
        ready = true
        render()
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        guard webView === view else { return }
        ready = false
        webView.reload()
    }

    private func render() {
        guard ready, source != nil else { return }
        guard let view else { return }
        let update = update
        Task { @MainActor in
            do {
                _ = try await view.callAsyncJavaScript(
                    "window.render_message(update)",
                    arguments: ["update": update],
                    in: nil,
                    contentWorld: .page
                )
            } catch {
                NSLog("Markdown rendering failed: %@", error.localizedDescription)
            }
        }
    }

    func userContentController(_ userContentController: WKUserContentController, didReceive message: WKScriptMessage) {
        guard message.webView === view, message.frameInfo.isMainFrame, let value = message.body as? Double,
              value.isFinite, value > 0, value < 10_000_000 else { return }
        let next_height = max(1, CGFloat(value))
        if height.wrappedValue != next_height { height.wrappedValue = next_height }
        measured.wrappedValue = true
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        guard let url = navigationAction.request.url else {
            decisionHandler(.cancel)
            return
        }
        if navigationAction.navigationType == .linkActivated {
            if ["https", "http", "mailto"].contains(url.scheme?.lowercased() ?? "") {
                #if os(macOS)
                NSWorkspace.shared.open(url)
                #else
                UIApplication.shared.open(url)
                #endif
            }
            decisionHandler(.cancel)
            return
        }
        decisionHandler(url.isFileURL ? .allow : .cancel)
    }
}

#if os(macOS)
private final class MarkdownWebView: WKWebView {
    override func scrollWheel(with event: NSEvent) {
        if abs(event.scrollingDeltaY) >= abs(event.scrollingDeltaX) {
            nextResponder?.scrollWheel(with: event)
        } else {
            super.scrollWheel(with: event)
        }
    }
}
#endif
