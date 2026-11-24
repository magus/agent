import Foundation
import Observation
#if os(iOS)
import UIKit
#endif

enum ChatSheet: String, Identifiable {
    case connection, settings, devices, chatgpt, conversation_json
    var id: String { rawValue }
}

@MainActor @Observable
final class ChatModel {
    var sheet: ChatSheet?
    private(set) var creating = false
    var conversation = Conversation()
    var conversation_list: [ConversationRecord] = []
    var service_list: [Device] = []
    var enabled_device_list: [Device] = []
    var device_list: [Device] = []
    var model_option_list: [ModelOption] = []
    var default_device_id_list: [String] = []
    var default_model_settings = ModelSettings()
    /// True once the client has a first answer (or error) from the service.
    private(set) var loaded = false
    var local_device_id = ""
    /// Why computer use is off on this device, with the fix; nil when it is available.
    var local_computer_unavailable: String?
    var saving_devices = false
    var service_host = ""
    var service_name = ""
    var draft = "" {
        didSet { drafts.write(draft, key: draft_key) }
    }
    var selected_model = ""
    var selected_effort = ""
    var running = false
    var sending = false
    var error: String?
    /// The error the last failed refresh showed; a later successful refresh clears it if still shown.
    private var refresh_error: String?
    private(set) var initialized = false
    var notice = ""
    private(set) var pending: PendingMessage?

    var pending_entry: Entry? {
        guard let pending, pending.service == service_host,
              pending.conversation_id == conversation.id else { return nil }
        return pending.entry
    }

    var pending_failure: String? { pending_entry == nil ? nil : pending?.failure }

    var can_send: Bool {
        selecting_conversation_id == nil && !sending && !running && !conversation.has_pending_tools
            && pending == nil && !selected_model.isEmpty
            && !conversation.id.isEmpty && !draft.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private let client = Client()
    private var initialization_task: Task<Void, Never>?
    private var poll_task: Task<Void, Never>?
    private var models_host = ""
    private var model_save_count = 0
    private let drafts = ChatDrafts()
    private var draft_key = ""
    private var selecting_conversation_id: String?

    func run() async {
        if initialization_task == nil {
            initialization_task = Task { await initialize() }
        }
        await initialization_task?.value
    }

    private func initialize() async {
        do {
            let directory = try FileManager.default.url(
                for: .applicationSupportDirectory, in: .userDomainMask,
                appropriateFor: nil, create: true
            ).appendingPathComponent("Agent", isDirectory: true)
            try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
            #if os(iOS)
            let device_name = UIDevice.current.name
            #else
            let device_name = Host.current().localizedName ?? "Mac"
            #endif
            try await client.perform(["action": "initialize", "directory": directory.path, "name": device_name])
            initialized = true

            await refresh()
            poll_task = Task { await poll() }
        } catch {
            initialization_task = nil
            self.error = error.localizedDescription
        }
    }

    private func poll() async {
        defer { poll_task = nil }

        while !Task.isCancelled {
            do { try await Task.sleep(for: .milliseconds(250)) }
            catch { return }
            await refresh()
        }
    }

    func refresh() async {
        do {
            let snapshot = try await client.request(["action": "snapshot"], as: Snapshot.self)
            if let selecting_conversation_id, snapshot.conversation.id != selecting_conversation_id {
                return
            }
            let changed_conversation = conversation.id != snapshot.conversation.id || service_host != snapshot.service_host
            if conversation != snapshot.conversation { conversation = snapshot.conversation }
            notice = snapshot.notice
            if conversation_list != snapshot.conversation_list {
                conversation_list = snapshot.conversation_list
            }
            service_list = snapshot.service_list
            service_host = snapshot.service_host
            service_name = snapshot.service_name
            running = snapshot.running
            enabled_device_list = snapshot.target_list
            device_list = snapshot.available_target_list
            default_device_id_list = snapshot.default_target_list
            default_model_settings = snapshot.model_settings
            loaded = snapshot.revision > 0
            local_device_id = snapshot.local_device_id
            local_computer_unavailable = snapshot.local_unavailable_tools["computer"]

            let next_draft_key = service_host + "/" + conversation.id
            if !conversation.id.isEmpty && draft_key != next_draft_key {
                let preserve_initial_draft = draft_key.isEmpty && !draft.isEmpty
                draft_key = next_draft_key
                if preserve_initial_draft { drafts.write(draft, key: draft_key) }
                else { draft = drafts.read(draft_key) }
            }

            if !sending && pending != snapshot.pending { pending = snapshot.pending }
            if model_save_count == 0 || changed_conversation {
                selected_model = conversation.model
                selected_effort = conversation.reasoning_effort ?? ""
            }
            if !service_host.isEmpty && models_host != service_host {
                model_option_list = try await client.request(["action": "models"], as: [ModelOption].self)
                models_host = service_host
            }
            // Fall back to the first model only once a loaded conversation has none, so the picker
            // never flashes a model the conversation doesn't use.
            if selected_model.isEmpty && !conversation.id.isEmpty && conversation.model.isEmpty {
                selected_model = model_option_list.first?.model ?? ""
            }
            if refresh_error != nil {
                if error == refresh_error { error = nil }
                refresh_error = nil
            }
        } catch {
            self.error = error.localizedDescription
            refresh_error = self.error
        }
    }

    func perform(_ request: [String: Any]) async {
        do {
            try await client.perform(request)
            error = nil
            await refresh()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func create() async {
        guard !creating else { return }
        creating = true
        defer { creating = false }
        await perform(["action": "create"])
    }

    func select(_ id: String) async {
        guard id != conversation.id else { return }

        selecting_conversation_id = id
        draft_key = service_host + "/" + id
        draft = drafts.read(draft_key)
        conversation = Conversation(id: id)
        selected_model = ""
        selected_effort = ""
        enabled_device_list = []
        running = false

        do {
            try await client.perform(["action": "select", "id": id])
            guard selecting_conversation_id == id else { return }
            error = nil
        } catch {
            guard selecting_conversation_id == id else { return }
            self.error = error.localizedDescription
        }
        selecting_conversation_id = nil
        await refresh()
    }

    func pair(_ invitation: String) async throws {
        let invitation = invitation.trimmingCharacters(in: .whitespacesAndNewlines)
        try await client.perform(["action": "pair", "invitation": invitation])
        await refresh()
    }

    func send() {
        let prompt = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard can_send else { return }

        let settings = ModelSettings(model: selected_model,
                                     reasoning_effort: selected_effort.isEmpty ? nil : selected_effort)
        do {
            pending = try client.prepare_delivery(
                service: service_host, conversation_id: conversation.id,
                prompt: prompt, settings: settings
            )
            draft = ""
            retry_pending()
        } catch {
            self.error = error.localizedDescription
        }
    }

    func retry_pending() {
        guard pending != nil, !sending else { return }
        deliver(["action": "retry"])
    }

    private func deliver(_ request: [String: Any]) {
        sending = true
        error = nil
        pending?.failure = nil

        Task {
            var delivery_error: String?
            do {
                try await client.perform(request)
            } catch {
                delivery_error = error.localizedDescription
            }
            sending = false
            await refresh()
            if pending_failure == nil { error = delivery_error }
        }
    }

    func open_pending() async {
        guard let pending else { return }
        if service_host != pending.service {
            await perform(["action": "connect", "host": pending.service])
        }
        await select(pending.conversation_id)
    }

    func save_model() async {
        guard !conversation.id.isEmpty else { return }
        await perform([
            "action": "set_model", "service": service_host,
            "id": conversation.id, "settings": settings,
        ])
    }

    func choose_model(_ value: String) {
        selected_model = value
        let effort_list = model_option_list.first { $0.model == value }?.reasoning_effort_list ?? []
        if !effort_list.contains(selected_effort) { selected_effort = "" }
        save_selected_model()
    }

    func choose_effort(_ value: String) {
        selected_effort = value
        save_selected_model()
    }

    func toggle_device(_ device: Device) async {
        guard !saving_devices, !conversation.id.isEmpty else { return }
        saving_devices = true
        defer { saving_devices = false }

        var target_list = enabled_device_list.map(\.id)
        if target_list.contains(device.id) { target_list.removeAll { $0 == device.id } }
        else { target_list.append(device.id) }

        await perform([
            "action": "set_targets", "service": service_host,
            "id": conversation.id, "target_list": target_list,
        ])
    }

    func save_defaults(_ settings: ModelSettings, device_id_list: [String]) async throws {
        try await client.perform(["action": "set_model_defaults", "settings": settings.request_value])
        try await client.perform(["action": "set_device_defaults", "target_list": device_id_list])
        await refresh()
    }

    private func save_selected_model() {
        guard !conversation.id.isEmpty else { return }
        let request: [String: Any] = [
            "action": "set_model", "service": service_host,
            "id": conversation.id, "settings": settings,
        ]
        model_save_count += 1
        Task {
            await perform(request)
            model_save_count -= 1
            if model_save_count == 0 { await refresh() }
        }
    }

    private var settings: [String: Any] {
        var value: [String: Any] = ["model": selected_model]
        if !selected_effort.isEmpty { value["reasoning_effort"] = selected_effort }
        return value
    }
}
