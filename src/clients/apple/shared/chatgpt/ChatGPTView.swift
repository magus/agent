import SwiftUI

struct ChatGPTView: View {
    let service: String
    @State private var model = ChatGPTModel()
    @State private var hovering = false
    @State private var confirming_sign_out = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    if model.loading {
                        ProgressView().controlSize(.small).frame(minHeight: 22)
                    } else if let account = model.account {
                        LabeledContent("Signed in as") {
                            // An address must not wrap: iOS hyphenates it at large text sizes
                            // ("name@domain.-com"), which reads as a different address.
                            Text(account).textSelection(.enabled)
                                .lineLimit(1).minimumScaleFactor(0.5)
                        }
                        Button("Check connection") {
                            Task { await model.perform("check_model", service: service) }
                        }.disabled(model.pending || model.busy)
                        login_button("Sign in with another account")
                        // Signing out affects every device on this service, so it asks first.
                        Button("Sign out", role: .destructive) { confirming_sign_out = true }
                            .disabled(model.pending || model.busy)
                            .confirmationDialog(
                                "Sign out of ChatGPT?",
                                isPresented: $confirming_sign_out,
                                titleVisibility: .visible
                            ) {
                                Button("Sign out", role: .destructive) {
                                    Task { await model.perform("sign_out", service: service) }
                                }
                            } message: {
                                Text("Agent stops answering on every device paired with this service until someone signs in again.")
                            }
                    } else {
                        Text("Sign in once. Every device paired with this service uses the same account.")
                            .foregroundStyle(.secondary)
                        login_button("Continue with ChatGPT")
                    }
                }
                if let message = model.message { Text(message).textSelection(.enabled) }
                if let error = model.error { Text(error).foregroundStyle(.red).textSelection(.enabled) }
            }
            .formStyle(.grouped)
            .navigationTitle("ChatGPT")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        .task { await model.poll(service: service) }
        .onDisappear { Task { await model.cancel() } }
        #if os(macOS)
        .frame(width: 440, height: 320)
        #endif
    }

    private func login_button(_ title: String) -> some View {
        Button {
            Task {
                if model.pending { await model.cancel() }
                else { await model.start(service: service) }
            }
        } label: {
            HStack {
                if model.pending {
                    ProgressView().controlSize(.small).accessibilityHidden(true)
                }
                Text(model.pending ? (hovering ? "Cancel login" : "Waiting for login…") : title)
            }
        }
        .onHover { hovering = $0 }
        .accessibilityLabel(model.pending ? "Cancel login" : title)
        .disabled(model.busy)
    }
}
