# Direct device tools

Open **Agent → Devices** on each Mac. The list includes known and discovered devices. Connection
status appears beside each name. **Outgoing** controls access to that device. **Incoming** controls
its access to the device running Agent. Choose **Request access**, then **Allow** on the receiving
device. Approval grants control in that direction only.

**Remove** removes only the outgoing connection. **Revoke** removes only the incoming grant.
It also prevents repeated requests from prompting again. To accept a new request after declining
or revoking access, choose **Allow requests** in that device's Incoming column.
This allows another request, not control of the Mac.

The refresh icon scans for devices. **Add device** accepts an IP address or hostname, verifies the
endpoint, and shows its name without granting access. A hostname can include a port, such as
`joahs-mac-mini:47322` for the central service. Tailscale currently supplies discovery and address
verification; execution devices use port 47321. Hostname lookup can use its device list when system DNS fails.
Technical connection information is under **Connection details**.

The default device name is the macOS computer name. Saving a name sets an Agent-specific override.
**Use system name** clears that override. Changing a name does not change identity or permissions.

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
