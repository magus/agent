import Foundation

struct PendingMessage: Codable, Equatable {
    let service: String
    let conversation_id: String
    let message_id: String
    let prompt: String
    let settings: ModelSettings
    var failure: String?

    var entry: Entry { Entry(message_id: message_id, role: "You", text: prompt) }

}
