import Foundation

actor Client {
    func request<Result: Decodable>(_ request: [String: Any], as type: Result.Type) throws -> Result {
        try Self.exchange(request, as: type)
    }

    // This request saves locally. It must never perform network work.
    nonisolated func prepare_delivery(
        service: String,
        conversation_id: String,
        prompt: String,
        settings: ModelSettings
    ) throws -> PendingMessage {
        try Self.exchange([
            "action": "prepare_send", "service": service, "id": conversation_id,
            "prompt": prompt, "settings": settings.request_value,
        ], as: PendingMessage.self)
    }

    private nonisolated static func exchange<Result: Decodable>(
        _ request: [String: Any],
        as type: Result.Type
    ) throws -> Result {
        let data = try JSONSerialization.data(withJSONObject: request)
        let text = String(decoding: data, as: UTF8.self)
        guard let response = text.withCString({ agent_request($0) }) else {
            throw ClientError(message: "Agent did not return a response.")
        }
        defer { agent_string_free(response) }

        let response_data = Data(String(cString: response).utf8)
        let envelope = try JSONDecoder().decode(Response<Result>.self, from: response_data)
        guard envelope.ok, let result = envelope.result else {
            throw ClientError(message: envelope.error ?? "Request failed.")
        }
        return result
    }

    func perform(_ request: [String: Any]) throws {
        _ = try self.request(request, as: Empty.self)
    }
}

private struct Response<Result: Decodable>: Decodable {
    let ok: Bool
    let result: Result?
    let error: String?
}

private struct Empty: Decodable {}

struct ClientError: LocalizedError {
    let message: String
    var errorDescription: String? { message }
}
