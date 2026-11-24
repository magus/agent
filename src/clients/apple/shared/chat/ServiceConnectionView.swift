import SwiftUI

struct ServiceConnectionView: View {
    @Bindable var model: ChatModel
    @Environment(\.dismiss) private var dismiss
    @State private var invitation = ""
    @State private var connecting = false
    @State private var requested = false
    @State private var error: String?

    var body: some View {
        NavigationStack {
            Form {
                if !model.service_list.isEmpty {
                    Section("Services") {
                        ForEach(model.service_list) { service in
                            let connected = model.service_host == service.id
                            Button {
                                Task {
                                    await model.perform(["action": "connect", "host": service.id])
                                    if model.service_host == service.id { dismiss() }
                                }
                            } label: {
                                HStack {
                                    Text(service.name)
                                    Spacer()
                                    if connected {
                                        Image(systemName: "checkmark").fontWeight(.semibold)
                                    }
                                }
                                .contentShape(Rectangle())
                            }
                            .accessibilityAddTraits(connected ? .isSelected : [])
                        }
                    }
                }

                Section("Pair manually") {
                    TextField("Invitation", text: $invitation, axis: .vertical)
                        .lineLimit(2...4)
                        .autocorrectionDisabled()

                    Button("Request connection") {
                        connecting = true
                        error = nil

                        Task {
                            do {
                                try await model.pair(invitation)
                                invitation = ""
                                requested = true
                            } catch {
                                self.error = error.localizedDescription
                            }
                            connecting = false
                        }
                    }
                    .disabled(connecting || invitation.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)

                    if connecting { ProgressView() }
                    if requested { Text("Approve this device on the service host.").foregroundStyle(.secondary) }
                    if let error { Text(error).foregroundStyle(.red) }
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Connect to service")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
            }
        }
        #if os(macOS)
        .frame(width: 460, height: 360)
        #endif
    }
}
