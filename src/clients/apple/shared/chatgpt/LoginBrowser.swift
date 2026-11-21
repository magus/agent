import Foundation
#if os(macOS)
import AppKit
#else
import AuthenticationServices
import UIKit
#endif

@MainActor
final class LoginBrowser: NSObject {
    #if os(macOS)
    private weak var login_window: NSWindow?
    #endif
    #if os(iOS)
    private var session: ASWebAuthenticationSession?
    private var session_id: UUID?
    private var cancelled: (() -> Void)?
    #endif

    func open(_ url: URL, cancelled: @escaping () -> Void) -> Bool {
        #if os(macOS)
        login_window = NSApplication.shared.keyWindow
        return NSWorkspace.shared.open(url)
        #else
        let id = UUID()
        session_id = id
        self.cancelled = cancelled
        let session = ASWebAuthenticationSession(url: url, callbackURLScheme: nil) { [weak self] _, _ in
            Task { @MainActor in
                guard let self, self.session_id == id else { return }
                self.cancelled?()
            }
        }
        session.presentationContextProvider = self
        self.session = session
        return session.start()
        #endif
    }

    func finish(activate: Bool) {
        #if os(macOS)
        guard activate else { return }

        let configuration = NSWorkspace.OpenConfiguration()
        configuration.activates = true
        configuration.addsToRecentItems = false

        NSWorkspace.shared.openApplication(at: Bundle.main.bundleURL, configuration: configuration) {
            [weak self] _, error in
            Task { @MainActor in
                guard error == nil else { return }

                self?.login_window?.makeKeyAndOrderFront(nil)
            }
        }
        #else
        session_id = nil
        cancelled = nil
        session?.cancel()
        session = nil
        #endif
    }
}

#if os(iOS)
extension LoginBrowser: ASWebAuthenticationPresentationContextProviding {
    func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
        UIApplication.shared.connectedScenes
            .compactMap { $0 as? UIWindowScene }
            .flatMap(\.windows)
            .first(where: \.isKeyWindow) ?? ASPresentationAnchor()
    }
}
#endif
