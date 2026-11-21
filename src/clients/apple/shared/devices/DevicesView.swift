import SwiftUI

struct DevicesView: View {
    @State private var model = DeviceModel()
    @State private var host = ""
    @State private var invitation = ""
    @State private var showing_manual = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    HStack {
                        TextField("Device name", text: $model.name)
                        Button("Save") {
                            Task { await model.perform(["operation": "rename", "name": model.name]) }
                        }
                        .disabled(model.busy || model.name == model.snapshot.name || model.name.isEmpty)
                    }
                }

                Section("Devices") {
                    ForEach(model.snapshot.device_list) { device in
                        connection(device)
                    }
                    if model.snapshot.device_list.isEmpty {
                        Text("No devices connected").foregroundStyle(.secondary)
                    }
                }

                #if os(macOS)
                Section("Add device") {
                    HStack {
                        TextField("IP address or hostname", text: $host)
                            .autocorrectionDisabled()
                        Button("Find") {
                            Task { await model.perform(["operation": "find", "host": host]) }
                        }.disabled(model.busy || host.isEmpty)
                    }
                }
                #endif

                DisclosureGroup("Pair manually", isExpanded: $showing_manual) {
                    TextField("Invitation", text: $invitation, axis: .vertical)
                        .lineLimit(2...4).autocorrectionDisabled()
                    Button("Request connection") {
                        Task {
                            if await model.pair(invitation) { invitation = "" }
                        }
                    }.disabled(model.busy || invitation.isEmpty)

                    if model.snapshot.execution_device {
                        Button("Create invitation") { Task { await model.invite() } }
                            .disabled(model.busy)
                    }
                    if let invitation = model.invitation {
                        ShareLink("Share invitation", item: invitation)
                    }
                }

                if let error = model.error {
                    Text(error).foregroundStyle(.red).textSelection(.enabled)
                }
            }
            .formStyle(.grouped)
            .navigationTitle("Devices")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    Button("Refresh", systemImage: "arrow.clockwise") {
                        Task { await model.perform(["operation": "refresh"]) }
                    }.disabled(model.busy)
                }
            }
        }
        .task { await model.poll() }
        #if os(macOS)
        .frame(minWidth: 540, minHeight: 440)
        #endif
    }

    private func connection(_ device: DeviceConnection) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                Text(device.name)
                Spacer()
                if device.outgoing {
                    Text(device.status).font(.caption).foregroundStyle(.secondary)
                }
                Menu {
                    if device.discovered && !device.outgoing {
                        action("Request access", operation: "request_access", id: device.id)
                    }
                    if device.outgoing {
                        action("Remove outgoing access", operation: "remove_target", id: device.id)
                    }
                    if device.incoming {
                        action("Revoke incoming access", operation: "revoke_access", id: device.id)
                    }
                    if device.blocked {
                        action("Allow new requests", operation: "allow_requests", id: device.id)
                    }
                    Divider()
                    Text(device.address)
                    Text(device.id)
                } label: { Label("Connection", systemImage: "ellipsis") }
                .labelsHidden()
                .accessibilityLabel("Connection for \(device.name)")
                .menuStyle(.borderlessButton)
                .disabled(model.busy)
            }
            HStack {
                if device.outgoing { Label("Outgoing", systemImage: "arrow.up.right") }
                if device.incoming { Label("Incoming", systemImage: "arrow.down.left") }
            }.font(.caption).foregroundStyle(.secondary)

            if device.pending {
                HStack {
                    Text("Requests access").font(.caption)
                    Spacer()
                    action("Decline", operation: "decline", id: device.id)
                    action("Allow", operation: "approve", id: device.id)
                }.disabled(model.busy)
            }
        }
    }

    private func action(_ title: String, operation: String, id: String) -> some View {
        Button(title) {
            Task { await model.perform(["operation": operation, "id": id]) }
        }
    }
}
