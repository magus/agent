# Device discovery and directional access

## Context

The device screen mixes incoming and outgoing access and exposes connection details as editable
fields. The user wants discovery, explicit one-way approval, and clear feedback on both Macs.
The final state must allow the MacBook to control the mini, but not the reverse.

## Decisions

- Keep rustls TLS 1.3 and existing Keychain identities. Discovery does not authorize tools.
- Use Tailscale's local identity information to authenticate discovery before pinning certificates.
- Public discovery metadata travels inside Tailscale's encrypted tunnel. WhoIs checks the direct
  device address and stable node ID. A pinned TLS handshake then proves possession of the Agent key.
  Tool requests still require an approved client certificate. No tool route uses public discovery.
- Pairing also verifies the requesting endpoint's key through a pinned TLS connection. Discovery
  requests must come from that endpoint's Tailscale address.
- Do not add inference, an agent loop, or a certificate authority.

## Changes

- Added separate target removal, incoming revocation, and persistent decline decisions.
- Added automatic address detection, Tailscale discovery, and a public identity-only route.
- Reworked the screen into targets, incoming access, and available devices. Manual pairing and
  connection details are collapsed. Feedback stays beside the relevant action.
- Manual pairing opens a compact view instead of adding controls below the visible area. Empty
  feedback no longer adds blank lines. Target status refreshes independently of discovery.
- Declined devices cannot prompt again until the receiver allows new requests. Approval requires
  an actual incoming request. Discovery updates names only for the same verified Agent identity.

## Verification

- Tailscale reports both Macs online. Its WhoIs result identifies the mini with a stable node ID.
- All 22 tests passed, including directional removal, decline persistence, wrong identities,
  invitation replay, and private-key proof. Clippy passed with warnings denied.
- Both Macs discovered each other. Duplicate requests produced one pending row. Requests could
  not run tools before approval. Declining blocked repeated prompts. Explicit retry and approval
  succeeded. Manual invitation pairing also passed.
- Live checks proved incoming revocation preserves outgoing control and target removal preserves
  incoming control. The final one-way state passed a shell command and locked-desktop capture.
- Inspected the local screen, incoming request controls, visible Copied feedback, and error
  placement. Inspected the mini's screen through the existing full-desktop capture tool.
- The final signed update and explicit restarts preserved both identities, grants, and decline
  decisions. Both apps reconnected. MacBook → mini passed again; the reverse remained rejected.
  The mini still reported locked during the final desktop capture.
- Edited the MacBook name through the UI and verified discovery advertised it. Verified a mini
  rename updated the existing trusted target without losing access. Restored both original names.
- Stopping the mini produced an offline status beside its target row. Restart restored connection.
- The final binary is identical on both Macs, SHA-256
  `7dc1d8e9a9aeb65555e7e19d34c212b238eacd75bfaa98a68ff7a4b3bd48c44a`.
  Apple accepted notarization, stapler validation passed, and the mini passed code-signature and
  Gatekeeper checks. Installed at `dist/Agent.app` locally and
  `/Users/joah/Applications/Agent.app` on the mini.
- One earlier mini launch during bundle replacement needed a second launch. Waiting for the old
  process to exit before replacing and opening the bundle passed on the final update and restart.
