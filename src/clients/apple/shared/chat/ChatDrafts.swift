import Foundation

final class ChatDrafts {
    private let defaults: UserDefaults
    private let storage_key = "chat_draft_by_conversation"

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func read(_ key: String) -> String {
        defaults.dictionary(forKey: storage_key)?[key] as? String ?? ""
    }

    func write(_ text: String, key: String) {
        guard !key.isEmpty else { return }
        var draft_by_key = defaults.dictionary(forKey: storage_key) as? [String: String] ?? [:]
        if text.isEmpty { draft_by_key.removeValue(forKey: key) }
        else { draft_by_key[key] = text }
        defaults.set(draft_by_key, forKey: storage_key)
    }

}
