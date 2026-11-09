# Central conversations and device access

## Context

Conversations must remain available from every connected device when a client closes or disconnects.
Messages need to identify their sender. Tool access must follow explicit conversation grants.
This work began with conversation moves, then changed to a central service for history, model
execution, and ChatGPT authentication. The mini hosts the first service.

## Decisions

- Replace conversation moves with central ownership. Device apps provide the UI and execution tools.
  Preserve stable conversation IDs, sender context, screenshots, and original tool request IDs.
- Keep one refresh owner. The central service holds provider credentials and makes model requests.
  Clients and execution devices receive no provider tokens.
- Keep service registration separate from conversation device access. Copy creation defaults from
  the sending device. Opening a conversation or changing defaults cannot expand existing access.
- Use devices in the UI. Every tool call must choose an enabled device explicitly. Ordinary chat
  needs no device grant. "Here" refers to the device that sent that message, not the service host.
- Keep Enabled devices visible in chat. Put defaults for future conversations in Conversation settings.
- The user accepted full shell access for this version. Same-user shell commands can alter local
  Agent files and control sockets. Service checks prevent normal unauthorized dispatch but do not
  isolate a hostile agent with shell access to the service host. A future separate service machine
  protects service files; client-local control sockets remain accessible to that client's shell.

## Changes

- Added durable conversation records, copied creation defaults, explicit device grants, and
  authorization held through tool dispatch. Duplicate create and send requests preserve IDs and
  do not repeat execution. Conflicting retries are rejected.
- Added `--service`, `--service-device`, and `--service-install`. The service runs as a separate
  per-user launchd process with its own identity, Keychain entry, runtime directory, and port 47322.
  The existing device endpoint remains separate. The service rejects direct tool requests.
- Added authenticated list, create, get, send, stop, retrieve, model, account, login, and permission
  operations. There is one loop instance per conversation. UI calls run in the background.
- Converted chat to a service client. Pending sends pin their service and request IDs. Screenshots
  are cached locally and omitted from later responses when already present.
- Central login starts in the client browser. Its callback forwards the code over authenticated TLS.
  Only the service holds the PKCE verifier, exchanges the code, and saves or refreshes tokens.
- Removed move packets, ownership transfers, the move model tool, and Resolve move UI.
- Removed the preferred tool device from saved state, message submission, and the UI. Old saved
  values are ignored. This replaces an interim fix that merely cleared revoked preferred devices.
  That stale preference had incorrectly blocked ordinary messages after a permission change.
- Added Enabled devices and Edit devices to chat, plus Agent → Conversation settings for defaults.
- Updated the agent-core and central-service plans. Consolidated the eight active logs for this
  work, retaining the experiments and their limits below. Older committed logs were left unchanged.

## Verification

- The earlier move implementation passed 31 tests covering durable prepare/commit, duplicate and
  changed requests, ownership checks, screenshots, and pending tool identity. Live movement stopped
  before ownership changed because the mini lacked ChatGPT login. Successful live transfer and
  interrupted handoff were never verified. This approach was superseded, not completed.
- Sender-context proof ran `hostname` on the sending MacBook even when the old preferred device was
  the mini. The response correctly distinguished sender and loop host.
- The shared-token probe ran only after explicit user approval. An initial attempt stopped at a
  denied Keychain write before inference. The approved retry kept refresh tokens on the MacBook,
  sent access tokens through encrypted SSH stdin, and kept secrets out of files and tool output.
- Before refresh, both Macs concurrently used one access token. After one serialized refresh, both
  Macs concurrently used both the old and new access tokens. All six inference requests completed
  with HTTP 200 and correctly answered `17 * 23` as `391`. Timings were 1623–3532 milliseconds,
  excluding SSH setup. The replacement tokens were saved before further requests; the returned
  access lifetime was 3600 seconds. Agent restarted and loaded its model catalog afterward.
- That probe establishes immediate concurrent use across one refresh only. It does not establish
  long-term behavior, expiry behavior, or selective revocation. Provider documentation requires
  serialized refreshes and saving replacement tokens. See
  [sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions),
  [tokens](https://developers.openai.com/siwc/token-sharing-open-source/token-reference),
  [credential transfer](https://developers.openai.com/siwc/token-sharing-open-source/self-hosted-vms),
  and [refresh errors](https://developers.openai.com/siwc/token-sharing-open-source/errors-and-recovery).
  The implemented central service does not distribute tokens.
- The user completed central login from the MacBook and confirmed Agent regained focus. Both
  clients report the same account and run model requests without separate device logins.
- Both clients create, list, and open the same conversations. Mini creation defaults remain
  mini-only when viewed from the MacBook. The earlier 41-entry history and four screenshots were
  preserved under their original ID; original local history files remain untouched.
- Closing the MacBook UI did not interrupt a running central task. Reopening showed identical
  history with one copy of the message. Restarting the central service preserved a pending mini
  request, which was retrieved using its original ID without resubmission.
- Two concurrent sends accepted exactly one task. Stop cancelled a mini task and preserved its ID.
  Conversation grant and revocation checks passed with a real hostname call. Unpaired and revoked
  service requests fail. Revoking the mini client left the MacBook connected; access was restored.
- The service has a distinct identity from the mini execution device. Pairing was initially rejected
  by automatic review; identity and grant evidence supported the subsequent approved operation.
  Direct mini-to-MacBook execution remains denied.
- The central loop ran `hostname` on both Macs and captured the mini with `locked=true`.
  Both clients later sent messages to one conversation with zero enabled devices. Replies correctly
  identified each sender. Mini-only access was restored, then one explicit mini shell call returned
  `Joahs-Mac-mini.local`.
- Inspected the installed conversation list, history, enabled-device menu, and settings window.
  The restricted chat shows only the mini enabled. MacBook creation defaults show both devices.
- Final formatting, Clippy with warnings denied, all 32 Rust tests, and diff checks passed.
  Tests cover legacy preferred-device removal, explicit tool IDs, grants, persistence, and login
  ownership. An earlier sandboxed test run failed on denied loopback sockets; the authorized run passed.
- Apple accepted the latest build as `277858f5-a78b-463b-94bd-a9f4d5fac763`. Both Macs have the same
  installed binary hash. Signing, stapling, and Gatekeeper checks passed during installation.
  Existing identities, grants, and central login survived updates. Temporary deployment copies
  were removed; installed apps, the launchd job, and conversation history remain.
