import Foundation

struct Conversation: Decodable, Equatable {
    var id = ""
    var model = ""
    var reasoning_effort: String?
    var entry_list: [Entry] = []
    var status = ""

    var has_pending_tools: Bool {
        entry_list.contains { $0.request?.finished == false }
    }
}

struct Entry: Decodable, Equatable {
    var message_id: String?
    var role: String
    var text: String
    var image: String?
    var request: ToolRequest?
    var image_result_list: [SearchImage]?
}

struct ToolRequest: Decodable, Equatable {
    let id: String
    let host: String
    let status: String
    let finished: Bool
    let tool: JSONValue
}

struct Device: Decodable, Identifiable, Hashable {
    let id: String
    let name: String
}

struct ConversationRecord: Decodable, Identifiable, Equatable {
    let id: String
    let title: String
}

struct ModelOption: Decodable, Identifiable {
    let model: String
    let reasoning_effort_list: [String]
    var id: String { model }
}

struct Snapshot: Decodable {
    let pending: PendingMessage?
    let conversation: Conversation
    let notice: String
    let conversation_list: [ConversationRecord]
    let service_list: [Device]
    let service_host: String
    let service_name: String
    let running: Bool
    let target_list: [Device]
    let available_target_list: [Device]
    let default_target_list: [String]
    let model_settings: ModelSettings
    let local_device_id: String
    let local_unavailable_tools: [String: String]
}
