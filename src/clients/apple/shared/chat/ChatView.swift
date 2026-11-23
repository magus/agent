import SwiftUI

struct ChatView: View {
    @Bindable var model: ChatModel
    @State private var expanded_id_set: Set<String> = []
    @FocusState private var composer_focused: Bool
    @State private var showing_connection = false
    @State private var showing_json = false
    @State private var showing_settings = false
    @State private var showing_devices = false
    @State private var showing_chatgpt = false
    @State private var composer_height: CGFloat = 46
    #if os(macOS)
    @Environment(\.openWindow) private var open_window
    #endif

    var body: some View {
        NavigationSplitView {
            List(selection: Binding(
                get: { Optional(model.conversation.id) },
                set: { id in
                    if let id { Task { await model.select(id) } }
                }
            )) {
                ForEach(model.conversation_list) { conversation in
                    NavigationLink(value: conversation.id) {
                        Text(conversation.title)
                    }
                }
            }
            .navigationTitle("Conversations")

        } detail: {
            VStack(spacing: 0) {
                if !model.device_list.isEmpty {
                    ScrollView(.horizontal) {
                        HStack {
                            ForEach(model.device_list) { device in
                                let enabled = model.enabled_device_list.contains(device)
                                Button {
                                    Task { await model.toggle_device(device) }
                                } label: {
                                    HStack(spacing: 4) {
                                        Text(device.name)
                                        if computer_unavailable(device) != nil {
                                            Image(systemName: "exclamationmark.triangle.fill")
                                                .foregroundStyle(.orange)
                                        }
                                    }
                                    .font(.caption)
                                    .padding(.horizontal, 10).padding(.vertical, 6)
                                    .background(.quaternary, in: Capsule())
                                    .opacity(enabled ? 1 : 0.4)
                                }
                                .buttonStyle(.plain)
                                .disabled(model.saving_devices || model.conversation.id.isEmpty)
                                .accessibilityValue(enabled ? "Enabled" : "Disabled")
                                .accessibilityHint(computer_unavailable(device) ?? "")
                                .help(computer_unavailable(device) ?? "")
                            }
                        }.padding(.horizontal)
                    }.padding(.vertical, 8)
                }
                if let reason = model.local_computer_unavailable,
                   model.enabled_device_list.contains(where: { $0.id == model.local_device_id }) {
                    computer_unavailable_banner(reason)
                }
                ConversationScrollView(sent_message_id: model.pending_entry?.message_id) {
                    VStack(alignment: .leading, spacing: 20) {
                        ForEach(Array(model.conversation.entry_list.enumerated()), id: \.offset) { index, entry in
                            message(entry, index: index, active: model.running && index == model.conversation.entry_list.count - 1)
                        }
                        if let pending = model.pending_entry {
                            message(pending)
                            if let failure = model.pending_failure {
                                HStack {
                                    Text(failure).foregroundStyle(.secondary)
                                    Button("Retry") { model.retry_pending() }.disabled(model.sending)
                                }.font(.caption)
                            }
                        }
                        if (model.sending || model.running) && !model.conversation.has_pending_tools {
                            HStack(spacing: 8) {
                                ProgressView().controlSize(.small)
                                Text("Thinking").foregroundStyle(.secondary)
                            }
                        }
                        if !model.notice.isEmpty {
                            Text(model.notice)
                                .foregroundStyle(.secondary)
                                .textSelection(.enabled)
                        }
                        if model.conversation.has_pending_tools && !model.running {
                            Button("Retrieve results") {
                                Task { await model.perform(["action": "retrieve"]) }
                            }
                        }
                        Color.clear.frame(height: 1).id("latest-message")
                    }
                    .scrollTargetLayout()
                    .padding(20)
                    .frame(maxWidth: 900)
                    .frame(maxWidth: .infinity)
                }
                .id(model.service_host + "/" + model.conversation.id)
                composer
            }
            .navigationTitle("")
            .toolbar {
                ToolbarItem(placement: .primaryAction) {
                    Button("New conversation", systemImage: "square.and.pencil") {
                        Task { await model.perform(["action": "create"]) }
                    }
                    .disabled(!model.initialized)
                }
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        ForEach(model.service_list) { service in
                            Button(service.name) {
                                Task { await model.perform(["action": "connect", "host": service.id]) }
                            }
                        }
                        Divider()
                        Button("Connect to service…") { showing_connection = true }
                        Button("Devices…") { showing_devices = true }
                        Button("ChatGPT…") { showing_chatgpt = true }
                            .disabled(model.service_host.isEmpty)
                        Button("Conversation settings…") { showing_settings = true }
                            .disabled(model.service_host.isEmpty)
                    } label: {
                        Label("Service", systemImage: "network")
                    }
                    .disabled(!model.initialized)
                }
                ToolbarItem(placement: .primaryAction) {
                    Menu {
                        Button("View conversation JSON…") {
                            #if os(macOS)
                            open_window(id: "conversation-json", value: ConversationJSONTarget(service: model.service_host, id: model.conversation.id))
                            #else
                            showing_json = true
                            #endif
                        }
                    } label: {
                        Label("Conversation actions", systemImage: "ellipsis.circle")
                    }
                    .disabled(model.service_host.isEmpty || model.conversation.id.isEmpty)
                }
            }
        }
        .task {
            await model.run()
            guard model.initialized else { return }
            #if os(macOS)
            switch CommandLine.arguments.dropFirst().first {
            case "--setup": open_window(id: "desktop-permissions")
            case "--devices": showing_devices = true
            case "--chatgpt": showing_chatgpt = true
            default: break
            }
            #endif
        }
        #if !os(macOS)
        .sheet(isPresented: $showing_json) {
            ConversationJSONView(service: model.service_host, id: model.conversation.id)
        }
        #endif
        .sheet(isPresented: $showing_connection) {
            ServiceConnectionView(model: model)
        }
        .sheet(isPresented: $showing_settings) {
            ConversationSettingsView(model: model)
        }
        .sheet(isPresented: $showing_devices) { DevicesView() }
        .sheet(isPresented: $showing_chatgpt) {
            if model.service_host.isEmpty { ServiceConnectionView(model: model) }
            else { ChatGPTView(service: model.service_host) }
        }
    }

    private func computer_unavailable(_ device: Device) -> String? {
        device.id == model.local_device_id ? model.local_computer_unavailable : nil
    }

    private func computer_unavailable_banner(_ reason: String) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 8) {
            Image(systemName: "exclamationmark.triangle.fill").foregroundStyle(.orange)
            Text(reason).foregroundStyle(.secondary).textSelection(.enabled)
            Spacer(minLength: 0)
            #if os(macOS)
            Button("Desktop permissions…") { open_window(id: "desktop-permissions") }
            #endif
        }
        .font(.caption)
        .padding(.horizontal)
        .padding(.bottom, 8)
    }

    private func message(_ entry: Entry, index: Int? = nil, active: Bool = false) -> some View {
        HStack(alignment: .top) {
            if entry.role == "You" { Spacer(minLength: 40) }
            Group {
                if entry.role == "Search", let index {
                    SearchSourcesView(query: entry.text, service: model.service_host, conversation_id: model.conversation.id,
                                      entry_index: index, running: model.running, image_list: entry.image_result_list ?? [])
                } else if let request = entry.request {
                    ToolResultView(entry: entry, request: request,
                        device_name: model.device_list.first(where: { $0.id == request.host })?.name ?? request.host,
                        expanded: Binding(
                        get: { expanded_id_set.contains(request.id) },
                        set: { expanded in
                            if expanded { expanded_id_set.insert(request.id) }
                            else { expanded_id_set.remove(request.id) }
                        }
                    ))
                } else {
                    Group {
                        if entry.role == "You" {
                            Text(entry.text).textSelection(.enabled)
                        } else {
                            let urls = entry.role == "Agent" && entry.text.contains("![")
                                ? model.conversation.entry_list.prefix(index ?? 0).flatMap { row in
                                    (row.image_result_list ?? []).flatMap { [$0.image_url, $0.thumbnail_url].compactMap { $0 } }
                                }
                                : []
                            MarkdownContent(source: entry.text, active: active, image_urls: urls).equatable()
                        }
                    }
                        .padding(entry.role == "You" ? 12 : 0)
                        .background {
                            if entry.role == "You" {
                                RoundedRectangle(cornerRadius: 18).fill(.tint.opacity(0.15))
                            }
                        }
                }
            }
            if entry.role != "You" { Spacer(minLength: 0) }
        }
    }

    private var composer: some View {
        VStack(alignment: .trailing, spacing: 10) {
            if model.pending != nil && model.pending_entry == nil && !model.sending {
                Button("Open unsent message") { Task { await model.open_pending() } }
            }
            if let error = model.error {
                Text(error).font(.caption).foregroundStyle(.red).textSelection(.enabled)
            }
            ModelControls(model: model)

            HStack(alignment: .bottom) {
                #if os(macOS)
                ComposerTextInput(text: $model.draft, height: $composer_height, send: model.send)
                    .frame(height: composer_height)
                    .overlay(alignment: .topLeading) {
                        if model.draft.isEmpty {
                            Text("Message Agent…")
                                .foregroundStyle(.tertiary)
                                .padding(.top, 2)
                                .allowsHitTesting(false)
                        }
                    }
                #else
                TextField("Message Agent…", text: $model.draft, axis: .vertical)
                    .textFieldStyle(.plain).lineLimit(2...8)
                    .focused($composer_focused)
                #endif
                Button {
                    if model.running && model.draft.isEmpty {
                        Task { await model.perform(["action": "stop"]) }
                    } else { model.send() }
                } label: {
                    Image(systemName: model.running && model.draft.isEmpty ? "stop.fill" : "arrow.up")
                        .frame(width: 26, height: 26)
                }
                .buttonStyle(.borderedProminent).buttonBorderShape(.circle)
                .accessibilityLabel(model.running && model.draft.isEmpty ? "Stop" : "Send")
                .disabled(!(model.running && model.draft.isEmpty) && !model.can_send)
            }
            .padding(14)
            .background(.background.secondary, in: RoundedRectangle(cornerRadius: 24))
        }
        .padding(16)
    }
}
