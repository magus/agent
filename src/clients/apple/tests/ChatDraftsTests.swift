import Foundation

@main
enum ChatDraftsTests {
    static func main() throws {
        let suite = "AgentDraftTests." + UUID().uuidString
        let defaults = UserDefaults(suiteName: suite)!
        defer { defaults.removePersistentDomain(forName: suite) }

        let first = ChatDrafts(defaults: defaults)
        first.write("Draft A", key: "service/chat-a")
        first.write("Draft B", key: "service/chat-b")
        let reopened = ChatDrafts(defaults: defaults)
        precondition(reopened.read("service/chat-a") == "Draft A")
        precondition(reopened.read("service/chat-b") == "Draft B")
        precondition(reopened.read("different-service/chat-a").isEmpty)
        precondition(reopened.read("service/chat-a") == "Draft A")
        reopened.write("", key: "service/chat-a")
        precondition(first.read("service/chat-a").isEmpty)
        precondition(first.read("service/chat-b") == "Draft B")
        print("Draft isolation and restoration passed")
    }
}
