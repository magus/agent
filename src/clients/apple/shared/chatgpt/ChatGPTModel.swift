import Foundation
import Observation

@MainActor @Observable
final class ChatGPTModel {
    var account: String?
    var loading = true
    var pending = false
    var busy = false
    var message: String?
    var error: String?
    private let client = Client()
    private let browser = LoginBrowser()
    private var opened_url: String?
    private var attempt_id: String?

    func load(service: String) async {
        defer { loading = false }
        do {
            let result = try await client.request(
                ["action": "account", "service": service, "operation": "status"],
                as: Account.self
            )
            account = result.account
        } catch { self.error = error.localizedDescription }
    }

    func poll(service: String) async {
        await load(service: service)
        while !Task.isCancelled {
            do {
                let status = try await client.request(["action": "login_status"], as: LoginStatus.self)
                if status.service == service && (attempt_id == nil || attempt_id == status.id) {
                    attempt_id = status.id
                    let finished = pending && !status.pending
                    pending = status.pending
                    if let url = status.url, url != opened_url {
                        guard let destination = URL(string: url), destination.scheme == "https",
                              destination.host == "auth.openai.com" else {
                            throw ClientError(message: "Unexpected login destination.")
                        }
                        opened_url = url
                        let opened = browser.open(destination) { [weak self] in
                            Task { await self?.cancel() }
                        }
                        if !opened {
                            await cancel()
                            throw ClientError(message: "Could not open ChatGPT login.")
                        }
                    }
                    if finished {
                        error = status.error
                        if status.completed { account = status.account }
                        browser.finish(activate: status.completed)
                    }
                }
                try await Task.sleep(for: .milliseconds(200))
            } catch is CancellationError { return }
            catch {
                self.error = error.localizedDescription
                return
            }
        }
    }

    func start(service: String) async {
        guard !pending, !busy else { return }
        pending = true
        let id = UUID().uuidString
        attempt_id = id
        opened_url = nil
        message = nil
        error = nil
        do { try await client.perform(["action": "login_start", "service": service, "id": id]) }
        catch {
            pending = false
            self.error = error.localizedDescription
        }
    }

    func cancel() async {
        guard pending else { return }
        browser.finish(activate: false)
        do { try await client.perform(["action": "login_cancel"]) }
        catch { self.error = error.localizedDescription }
    }

    func perform(_ operation: String, service: String) async {
        guard !busy, !pending else { return }
        busy = true
        message = nil
        error = nil
        defer { busy = false }

        do {
            _ = try await client.request(
                ["action": "account", "service": service, "operation": operation],
                as: AccountResult.self
            )
            message = operation == "check_model" ? "Connected" : nil
            await load(service: service)
        } catch { self.error = error.localizedDescription }
    }
}

private struct Account: Decodable {
    let account: String?
}

private struct AccountResult: Decodable {
    let message: String?
}

private struct LoginStatus: Decodable {
    let id: String
    let service: String
    let pending: Bool
    let url: String?
    let account: String?
    let error: String?
    let completed: Bool
}
