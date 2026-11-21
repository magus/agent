import SwiftUI

struct ChatGPTView: View {
    let service: String
    @State private var model = ChatGPTModel()
    @State private var hovering = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    Group {
                        if model.loading { ProgressView().controlSize(.small) }
                        else { Text(model.account ?? "Not signed in").textSelection(.enabled) }
                    }.frame(minHeight: 22)
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
                            Text(button_title)
                        }
                    }
                    .onHover { hovering = $0 }
                    .accessibilityLabel(model.pending ? "Cancel login" : "Continue with ChatGPT")
                    .disabled(model.busy || model.loading)

                    if model.account != nil {
                        Button("Check connection") {
                            Task { await model.perform("check_model", service: service) }
                        }.disabled(model.pending || model.busy)
                        Button("Sign out", role: .destructive) {
                            Task { await model.perform("sign_out", service: service) }
                        }.disabled(model.pending || model.busy)
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

    private var button_title: String {
        guard model.pending else { return "Continue with ChatGPT" }
        return hovering ? "Cancel login" : "Waiting for login…"
    }
}
