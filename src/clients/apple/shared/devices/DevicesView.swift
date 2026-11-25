import SwiftUI

struct DevicesView: View {
    @State private var model = DeviceModel()
    @State private var host = ""
    @State private var invitation = ""
    @State private var showing_manual = false
    @State private var removal: DeviceRemoval?
    @State private var refreshing = false
    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section("This device") {
                    HStack {
                        TextField("Name", text: $model.name)
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
            // Removing access can only be undone by pairing again, so it asks first.
            .confirmationDialog(
                removal?.title ?? "",
                isPresented: Binding(get: { removal != nil }, set: { if !$0 { removal = nil } }),
                titleVisibility: .visible,
                presenting: removal
            ) { removal in
                Button(removal.button, role: .destructive) {
                    Task { await model.perform(["operation": removal.operation, "id": removal.id]) }
                }
            } message: { removal in
                Text(removal.message)
            }
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Done") { dismiss() }
                }
                ToolbarItem(placement: .primaryAction) {
                    refresh_button
                }
            }
        }
        .task { await model.poll() }
        #if os(macOS)
        .frame(minWidth: 540, minHeight: 440)
        #endif
    }

    /// A scan usually finishes in well under a second; the spinner stays up for at least half a
    /// second so the tap visibly did something.
    @ViewBuilder private var refresh_button: some View {
        let button = Button("Refresh", systemImage: "arrow.clockwise") {
            Task {
                refreshing = true
                async let shown: Void? = try? Task.sleep(for: .milliseconds(500))
                await model.perform(["operation": "refresh"])
                _ = await shown
                refreshing = false
            }
        }
        .disabled(model.busy)
        #if os(macOS)
        // The Mac toolbar shows the title too, so the spinner covers the button instead of
        // replacing it and Done keeps its place.
        button
            .disabled(refreshing)
            .opacity(refreshing ? 0 : 1)
            .overlay {
                if refreshing {
                    ProgressView().controlSize(.small).accessibilityLabel("Refreshing")
                }
            }
        #else
        // iOS toolbars ignore modifiers on the item, so the spinner takes the icon's place.
        if refreshing {
            ProgressView().accessibilityLabel("Refreshing")
        } else {
            button
        }
        #endif
    }

    private func connection(_ device: DeviceConnection) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                // Name and status side by side when they fit; at large text sizes the status moves
                // under the name instead of being hyphenated beside it.
                ViewThatFits(in: .horizontal) {
                    HStack {
                        Text(device.name).fixedSize()
                        Spacer()
                        if device.outgoing { status(device) }
                    }
                    VStack(alignment: .leading, spacing: 2) {
                        Text(device.name)
                        if device.outgoing { status(device) }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                }
                Menu {
                    if device.discovered && !device.outgoing {
                        action("Request access", operation: "request_access", id: device.id)
                    }
                    if device.outgoing {
                        Button("Stop using this device") {
                            removal = DeviceRemoval(
                                id: device.id, operation: "remove_target",
                                title: "Stop using \(device.name)?", button: "Stop using",
                                message: "This device can't use \(device.name) until you pair them again."
                            )
                        }
                    }
                    if device.incoming {
                        Button("Revoke its access") {
                            removal = DeviceRemoval(
                                id: device.id, operation: "revoke_access",
                                title: "Revoke access for \(device.name)?", button: "Revoke access",
                                message: "\(device.name) can't use this device until you pair them again."
                            )
                        }
                    }
                    if device.blocked {
                        action("Allow new requests", operation: "allow_requests", id: device.id)
                    }
                    Divider()
                    Text(device.address)
                    Text(device.id)
                } label: {
                    // Named on the icon too: after a redraw, macOS can name the menu from the
                    // symbol ("More") instead of its accessibility label.
                    Image(systemName: "ellipsis.circle")
                        .accessibilityLabel("Options for \(device.name)")
                }
                .menuIndicator(.hidden)
                .fixedSize()
                .accessibilityLabel("Options for \(device.name)")
                #if os(macOS)
                .menuStyle(.borderlessButton)
                #endif
                .disabled(model.busy)
            }
            HStack(spacing: 12) {
                if device.outgoing { Label("This device can use it", systemImage: "arrow.up.right") }
                if device.incoming { Label("It can use this device", systemImage: "arrow.down.left") }
                if !device.outgoing && !device.incoming && !device.pending {
                    Text(device.discovered ? "No access. Request access from the options menu." : "No access")
                }
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

    private func status(_ device: DeviceConnection) -> some View {
        Text(device.status).font(.caption).foregroundStyle(.secondary).fixedSize()
    }

    private func action(_ title: String, operation: String, id: String) -> some View {
        Button(title) {
            Task { await model.perform(["operation": operation, "id": id]) }
        }
    }
}

private struct DeviceRemoval {
    let id: String
    let operation: String
    let title: String
    let button: String
    let message: String
}
