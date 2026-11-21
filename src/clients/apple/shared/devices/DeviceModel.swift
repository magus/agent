import Foundation
import Observation

@MainActor @Observable
final class DeviceModel {
    var snapshot = DeviceList()
    var name = ""
    var busy = false
    var error: String?
    var invitation: String?
    private let client = Client()
    private var loaded = false

    func poll() async {
        while !Task.isCancelled {
            await refresh()
            do { try await Task.sleep(for: .seconds(1)) }
            catch { return }
        }
    }

    func refresh() async {
        do {
            let updated = try await client.request(["action": "device_view"], as: DeviceList.self)
            if updated != snapshot { snapshot = updated }
            if !loaded {
                name = snapshot.name
                loaded = true
            }
        } catch { self.error = error.localizedDescription }
    }

    func perform(_ request: [String: Any]) async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }

        do {
            try await client.perform(["action": "device_action", "request": request])
            await refresh()
        } catch { self.error = error.localizedDescription }
    }

    func invite() async {
        guard !busy else { return }
        busy = true
        error = nil
        defer { busy = false }

        do {
            let result = try await client.request(
                ["action": "device_action", "request": ["operation": "invite"]],
                as: Invitation.self
            )
            invitation = result.invitation
        } catch { self.error = error.localizedDescription }
    }

    func pair(_ invitation: String) async -> Bool {
        guard !busy else { return false }
        busy = true
        error = nil
        defer { busy = false }

        do {
            try await client.perform([
                "action": "pair",
                "invitation": invitation.trimmingCharacters(in: .whitespacesAndNewlines),
            ])
            await refresh()
            return true
        } catch {
            self.error = error.localizedDescription
            return false
        }
    }
}

private struct Invitation: Decodable {
    let invitation: String
}
