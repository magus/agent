import Foundation

struct ModelSettings: Codable, Equatable {
    var model = ""
    var reasoning_effort: String?

    var request_value: [String: Any] {
        var value: [String: Any] = ["model": model]
        if let reasoning_effort, !reasoning_effort.isEmpty {
            value["reasoning_effort"] = reasoning_effort
        }
        return value
    }
}
