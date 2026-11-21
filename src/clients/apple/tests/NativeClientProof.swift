import SwiftUI

// Built only by the Simulator integration proof. Credentials stay in the app container.
@main
struct NativeClientProof: App {
    @State private var model = ChatModel()

    var body: some Scene {
        WindowGroup {
            ChatView(model: model)
                .task { await run_proof() }
        }
    }

    @MainActor
    private func run_proof() async {
        let client = Client()
        let directory = FileManager.default.urls(for: .documentDirectory, in: .userDomainMask)[0]
        let result_file = directory.appendingPathComponent("proof-result.json")

        func record(_ result: [String: String]) throws {
            let data = try JSONSerialization.data(withJSONObject: result)
            try data.write(to: result_file, options: .atomic)
        }

        do {
            // ChatView initializes the same bridge used by the production client.
            var ready = false
            for _ in 0..<100 {
                if let _ = try? await client.request(["action": "devices"], as: ProofDevice.self) {
                    ready = true
                    break
                }
                try await Task.sleep(for: .milliseconds(100))
            }
            guard ready else { throw ClientError(message: "Client did not initialize") }

            let invitation_file = directory.appendingPathComponent("proof-invitation.txt")
            if FileManager.default.fileExists(atPath: invitation_file.path) {
                let invitation = try String(contentsOf: invitation_file, encoding: .utf8)
                try await model.pair(invitation)
                try FileManager.default.removeItem(at: invitation_file)
            }

            let device = try await client.request(["action": "devices"], as: ProofDevice.self)
            guard device.tool_list.isEmpty else { throw ClientError(message: "iPhone advertises execution tools") }
            try record(["stage": "awaiting_approval", "device_id": device.id])

            for _ in 0..<240 {
                await model.refresh()
                if !model.service_list.isEmpty { break }
                try await Task.sleep(for: .seconds(1))
            }
            guard let service = model.service_list.first else { throw ClientError(message: "Service approval not received") }
            await model.perform(["action": "connect", "host": service.id])
            guard !model.conversation_list.isEmpty else { throw ClientError(message: "Shared history is empty") }

            await model.perform(["action": "create"])
            guard !model.conversation.id.isEmpty else { throw ClientError(message: "Conversation was not created") }
            model.selected_model = model.model_option_list.first(where: { $0.model == "gpt-6-luna" })?.model
                ?? model.model_option_list.first?.model ?? ""
            let option = model.model_option_list.first { $0.model == model.selected_model }
            model.selected_effort = option?.reasoning_effort_list.contains("low") == true ? "low" : ""
            await model.save_model()
            guard model.conversation.model == model.selected_model,
                  (model.conversation.reasoning_effort ?? "") == model.selected_effort else {
                throw ClientError(message: "Model settings were not saved")
            }
            model.draft = "Reply with exactly: Native iPhone connection works. Do not use tools."
            model.send()

            for _ in 0..<120 {
                try await Task.sleep(for: .seconds(1))
                await model.refresh()
                let received = model.conversation.entry_list.contains {
                    $0.role != "You" && $0.text.contains("Native iPhone connection works.")
                }
                if received && !model.running && !model.sending {
                    try await prove_tool()
                    try record(["stage": "passed", "conversation_id": model.conversation.id,
                                "model": model.selected_model, "effort": model.selected_effort])
                    return
                }
            }
            throw ClientError(message: model.error ?? "No model response received")
        } catch {
            try? record(["stage": "failed", "error": error.localizedDescription])
        }
    }

    @MainActor
    private func prove_tool() async throws {
        guard let mini = model.device_list.first(where: { $0.name.localizedCaseInsensitiveContains("mini") }) else {
            throw ClientError(message: "Mini execution device not found")
        }
        await model.perform([
            "action": "set_targets", "service": model.service_host,
            "id": model.conversation.id, "target_list": [mini.id],
        ])
        guard model.enabled_device_list.map(\.id) == [mini.id] else {
            throw ClientError(message: "Conversation device grant was not saved")
        }

        model.draft = "Use the shell tool to run hostname once on device \(mini.id). Report the output."
        model.send()

        for _ in 0..<120 {
            try await Task.sleep(for: .seconds(1))
            await model.refresh()
            let received = model.conversation.entry_list.contains {
                $0.request?.host == mini.id && $0.request?.finished == true
                    && $0.text.contains("Joahs-Mac-mini.local")
            }
            if received && !model.running && !model.sending { return }
        }
        throw ClientError(message: model.error ?? "Authorized shell result not received")
    }
}

private struct ProofDevice: Decodable {
    let id: String
    let tool_list: [String]
}
