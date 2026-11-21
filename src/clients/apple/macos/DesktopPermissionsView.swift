#if os(macOS)
import SwiftUI

struct DesktopPermissionsView: View {
    @State private var permissions = DesktopPermissions()
    @State private var preview_path: String?
    @State private var capturing = false
    @State private var capture_task: Task<Void, Never>?
    @State private var error: String?
    private let setup = DesktopSetup()

    var body: some View {
        Form {
            LabeledContent("Accessibility") {
                if permissions.accessibility {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                        .accessibilityLabel("Enabled")
                } else {
                    Button("Open Settings") { Task { await open_settings(accessibility: true) } }
                }
            }
            LabeledContent("Screen Recording") {
                if permissions.screen_recording {
                    Image(systemName: "checkmark.circle.fill").foregroundStyle(.green)
                        .accessibilityLabel("Enabled")
                } else {
                    Button("Open Settings") { Task { await open_settings(accessibility: false) } }
                }
            }
            Button("Check capture") {
                capturing = true
                error = nil
                capture_task = Task {
                    defer {
                        capturing = false
                        capture_task = nil
                    }
                    do {
                        clear_preview()
                        let path = try await setup.capture()
                        if Task.isCancelled {
                            try? FileManager.default.removeItem(atPath: path)
                            return
                        }
                        preview_path = path
                    } catch {
                        if !Task.isCancelled { self.error = error.localizedDescription }
                    }
                }
            }
            .disabled(!permissions.granted || capturing)

            if capturing { ProgressView() }
            if let error { Text(error).foregroundStyle(.red).textSelection(.enabled) }
            if let preview_path { ScreenshotView(path: preview_path) }
        }
        .formStyle(.grouped)
        .frame(minWidth: 460, minHeight: 300)
        .task {
            while !Task.isCancelled {
                do {
                    permissions = try await setup.permissions()
                    try await Task.sleep(for: .seconds(2))
                } catch is CancellationError { return }
                catch {
                    self.error = error.localizedDescription
                    return
                }
            }
        }
        .onDisappear {
            capture_task?.cancel()
            clear_preview()
        }
    }

    private func open_settings(accessibility: Bool) async {
        do {
            if accessibility { try await setup.open_accessibility() }
            else { try await setup.open_screen_recording() }
        } catch { self.error = error.localizedDescription }
    }

    private func clear_preview() {
        if let preview_path { try? FileManager.default.removeItem(atPath: preview_path) }
        preview_path = nil
    }
}
#endif
