import SwiftUI

struct ConversationSettingsView: View {
    let model: ChatModel
    @Environment(\.dismiss) private var dismiss
    @State private var settings = ModelSettings()
    @State private var device_id_set: Set<String> = []
    @State private var saving = false
    @State private var error: String?

    var body: some View {
        NavigationStack {
            Form {
                Section("New conversations") {
                    Picker("Model", selection: $settings.model) {
                        Text("Automatic").tag("")
                        ForEach(model.model_option_list) { option in
                            Text(option.model).tag(option.model)
                        }
                    }
                    .onChange(of: settings.model) { _, _ in
                        if !effort_list.contains(settings.reasoning_effort ?? "") {
                            settings.reasoning_effort = nil
                        }
                    }

                    Picker("Reasoning effort", selection: Binding(
                        get: { settings.reasoning_effort ?? "" },
                        set: { settings.reasoning_effort = $0.isEmpty ? nil : $0 }
                    )) {
                        Text("Automatic").tag("")
                        ForEach(effort_list, id: \.self) { effort in
                            Text(effort.capitalized).tag(effort)
                        }
                    }
                }

                Section("Default devices") {
                    ForEach(model.device_list) { device in
                        Toggle(device.name, isOn: Binding(
                            get: { device_id_set.contains(device.id) },
                            set: { enabled in
                                if enabled { device_id_set.insert(device.id) }
                                else { device_id_set.remove(device.id) }
                            }
                        ))
                    }
                }
                if let error { Text(error).foregroundStyle(.red) }
            }
            .formStyle(.grouped)
            .disabled(saving)
            .navigationTitle("Conversation settings")
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("Cancel") { dismiss() }.disabled(saving)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("Save") {
                        saving = true
                        Task {
                            do {
                                try await model.save_defaults(settings, device_id_list: device_id_set.sorted())
                                dismiss()
                            } catch {
                                self.error = error.localizedDescription
                            }
                            saving = false
                        }
                    }.disabled(saving)
                }
            }
        }
        .onAppear {
            settings = model.default_model_settings
            device_id_set = Set(model.default_device_id_list)
        }
        #if os(macOS)
        .frame(width: 480, height: 400)
        #endif
    }

    private var effort_list: [String] {
        model.model_option_list.first { $0.model == settings.model }?.reasoning_effort_list ?? []
    }
}
