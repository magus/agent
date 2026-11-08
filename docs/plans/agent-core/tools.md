# Direct device tools

Open **Agent → Devices** on each Mac. Agent detects its Tailscale address and uses port 47321.
Edit the device name if needed. Under **Available devices**, choose **Request access** on the
device you want to control. Choose **Allow** on that device. This grants shell and computer control
in one direction only. Reverse access needs a separate request and approval.

**Targets** lists outgoing connections. **Remove target** removes only that outgoing connection.
**Devices allowed to control this Mac** lists incoming grants. **Revoke access** removes only the
incoming grant. It also prevents repeated requests from prompting again. To accept a new request
after declining or revoking access, open **Connection details** and choose **Allow requests**.
This allows another request, not control of the Mac.

Use **Pair manually** if discovery is unavailable. Copy an invitation from the target, paste it on
the controller, and approve the request on the target. Invitations expire after 10 minutes.

The running app owns the identity in Keychain. The command client uses its private local socket.
No ChatGPT account or inference is involved.

## Discovery and identity

Agent reads Tailscale's device list and WhoIs identity. Public discovery metadata contains the
device name, certificate, and supported tools. It travels inside Tailscale's encrypted connection.
Agent checks the stable Tailscale device ID and direct address, then uses a pinned TLS handshake to
verify the Agent private key. Pairing checks the requesting address and verifies that endpoint's
private key too. A device name alone never grants access.

Tool requests use TLS 1.3 with approved client certificates through rustls. The protocol carries
length-prefixed JSON, not HTTP. Discovery and pairing do not grant tool access. Existing approved
certificates are not replaced when a new device appears at the same address.

## Client

Save a request as JSON, then run it through the installed app.

```sh
dist/Agent.app/Contents/MacOS/agent --device request.json
```

List devices with `{"operation":"list"}`. Use a unique device name or its full identity as `host`.
Check the target and its supported tools with this request.

```json
{"operation":"call","host":"mac-mini","request":{"operation":"info"}}
```

Submit a shell request with a fresh UUID. Submission returns before execution finishes.

```json
{
  "operation": "call",
  "host": "mac-mini",
  "request": {
    "operation": "submit",
    "id": "6df42171-9a01-4ac6-a96e-f330617bad4d",
    "tool": {
      "tool": "shell",
      "command": "pwd",
      "cwd": "/Users/joah",
      "timeout_seconds": 30
    }
  }
}
```

For computer use, replace `tool` with
`{"tool":"computer","action":{"operation":"capture"}}`.
Other actions follow the [desktop service requests](../../../src/agent/app/desktop_service/README.md#requests).
Computer results include the capture metadata and `image_base64` containing JPEG bytes.

Retrieve the result with the same `host` and
`"request":{"operation":"status","id":"6df42171-9a01-4ac6-a96e-f330617bad4d"}`.
Use `cancel` instead of `status` to request cancellation.

## Execution rules

- Reusing a request ID with the same owner and payload returns its existing status or result.
  A changed payload or owner is rejected. IDs and results survive app restarts.
- Commands continue when the client disconnects. If the target app stops before saving a result,
  the request becomes `interrupted` on restart and is not rerun.
- Cancellation kills a running shell's process group. A desktop action already dispatched cannot
  be undone; its result records `cancellation_requested` if cancellation arrived too late.
- Shell output is capped at 1 MiB per stream and reports truncation. Commands require an absolute
  working directory and a timeout of 1–86400 seconds. Processes that remain in the shell's process
  group stop with the command.
- Each device accepts at most 16 running requests. This is a resource limit, not desktop scheduling.
- Revocation prevents new requests, including result retrieval. It does not undo or stop work already
  accepted. Cancel that work before revoking if needed.
- Requests and results are private files under `~/Library/Application Support/Agent/requests/`.
  They can contain command output and screenshots. Automatic retention cleanup is not implemented.
