# Central service

## Status

Keep this checklist current. This is the active implementation plan. It replaces conversation moves.

- [x] Check OpenAI documentation for refresh ownership and remote credential use.
- [x] Prove concurrent access from two devices before and after a coordinated refresh.
- [x] Choose centralized loops and provider requests. Clients receive no provider tokens.
- [x] Choose the mini as the first service host, separate from its execution device process.
- [x] Implement service identity, protected account storage, startup, and configurable client connection.
- [x] Implement centrally stored conversations and per-conversation target permissions.
- [x] Implement shared conversation UI, target defaults, permissions, and client-initiated login.
- [x] Remove the superseded move workflow and preserve existing history.
- [x] Verify permissions, disconnects, concurrent submissions, Stop, restart, and pending results.
- [x] Sign, notarize, install, and verify on both Macs.

## Purpose

Sign in to ChatGPT once. Let authorized Agent devices run conversations without separate logins.
Keep conversation history and context available independently of any execution device.

The service owns account access, conversation storage, and agent loops. Devices provide the UI
and execute authorized tools. The device where the user types remains separate from the loop host.

## Account access

- Keep the refresh token in protected storage on the service. Devices never refresh it.
- Serialize refreshes per account session and save each replacement before serving more requests.
- Authenticate devices with the existing device identity and encrypted connection.
- Let the existing explicit device approval cover the account access needed for execution.
  Explain that access in the pairing flow without adding another approval step.
- Keep account delegation separate from permission to control another device. Access to the account
  must not create reverse shell or desktop access.

The service makes all provider requests. It does not distribute access tokens to devices.
Revoking client access stops new requests from that client. Execution-device grants and each
conversation’s allowed targets remain separate. Report the status of actions already dispatched.

## Conversation permissions

Register execution targets once. Store a separate allowed-target list on each conversation.
The service enforces that list on every tool submission. No model tool can edit it.
Full shell access still lets a process alter files owned by that OS user. While the service runs
under the same user on an execution target, this is not isolation against a hostile agent.
The user accepted this limit for now. A shell on a trusted UI device can also use its local control
socket to make user-level service requests. A future service host outside the execution targets
protects the service files, but does not by itself isolate those client sockets.

| Starting device | Initial allowed targets |
| --- | --- |
| MacBook | MacBook and mini |
| Mini, including future background agents | Mini |

Copy the starting device’s defaults when creating a conversation. Later default changes affect
only new conversations. Opening a conversation from another device does not expand its permissions.
Users can explicitly add and remove targets through the UI.

Every authorized UI client can list and continue all conversations. UI access alone does not
register the client’s machine as an execution target. The service host gets no implicit tool access.

## Conversation storage

- Store a stable conversation ID, ordered messages, sender device metadata, model settings,
  screenshots, tool requests and results, and execution status.
- Store large screenshots separately and reference them by stable ID. Avoid resending unchanged data.
- Keep tool request IDs and original target hosts. Reconnection must not repeat an action.
- Keep one active loop per conversation in the service. Clients never run that loop.
- Reopening the MacBook can reconnect its UI to a conversation running on the mini without moving
  the loop back or giving the mini permission to control the MacBook.
- Central storage does not authorize execution on every device. Check the conversation’s grants.

## Availability

The mini is the first service host. Its address is configurable so it can move elsewhere later. If it also runs
conversations, its restart interrupts both execution and the service. A separate always-on host
avoids that shared failure but adds another machine to maintain.

Model requests stop while the service is unavailable. Restore durable conversations after restart,
retrieve pending tool results using their original IDs, and report uncertain work without resubmitting
it. Closing a UI client does not stop a running conversation.

Start with one service and durable storage. Back up state and test restoration. Defer replication,
automatic failover, and scheduling until ordinary restart and recovery work.

## Completed token-sharing proof

Use the MacBook as the temporary refresh owner and the mini as a second inference client.

1. Stop competing refreshers for the selected session.
2. Run a small model request concurrently on both devices using the same access token.
3. Refresh once and persist the replacement in protected storage.
4. Test the old and replacement access tokens on both devices after refresh.
5. Restart Agent and verify its saved session still works.

Keep tokens out of arguments, files, logs, and reports. Send the mini’s access token only through
the encrypted connection and hold it in process memory. Do not copy the refresh token, revoke the
live account, or deliberately reuse an old refresh token.

A successful proof establishes observed behavior for this session. It does not establish a
permanent provider guarantee, independent device sessions, or selective revocation.

## Evidence

- [OpenAI accounts and sessions](https://developers.openai.com/siwc/token-sharing-open-source/profiles-and-sessions)
  requires coordinated refreshes and saving replacement tokens.
- [OpenAI remote-host guidance](https://developers.openai.com/siwc/token-sharing-open-source/self-hosted-vms)
  supports credential transfer but does not provide host-specific revocation for transferred sessions.
- [OpenAI token reference](https://developers.openai.com/siwc/token-sharing-open-source/token-reference)
  documents one-hour access tokens and replacement refresh tokens.
- [Shared-access proof and central service log](../../../logs/2026-11-09-15-37-central-service.md)
  records six successful inference requests. Both Macs used the same token concurrently before
  refresh. Both old and new access tokens worked on both Macs immediately after one refresh.
  The replacement was saved to Keychain, and Agent loaded its models after restart.

The proof established shared-token feasibility. The selected architecture instead keeps tokens
and loops in the service. No refresh token was sent to the mini during the proof.

Implementation is tracked in the [delivery log](../../../logs/2026-11-09-15-37-central-service.md).

## Startup

Install the signed app at a stable path on the service host, then run its installer.

```sh
~/Applications/Agent.app/Contents/MacOS/agent --service-install
```

This installs `~/Library/LaunchAgents/com.iamnoah.agent.service.plist`. The service starts at login,
continues while the screen is locked, and restarts after a process failure. It does not run before
that user logs in after a reboot. It runs separately from the desktop UI and Swift helper.

The service listens on tailnet port 47322. Execution devices use 47321. Pair clients with the service
identity, then register each execution device with the service. These are separate from direct
device-to-device control grants. The chat window selects a paired service.

Service state lives in `~/Library/Application Support/Agent/service`. Provider credentials use the
`com.iamnoah.agent.service.chatgpt` Keychain entry. Client selection and screenshot caches live under
`~/Library/Application Support/Agent/client`. Existing local history is retained under `chat`.

The service runs on the mini separately from Agent's desktop process. Closing either client leaves
central tasks running. Restarting the service interrupts its loops; use Retrieve results for saved
pending requests before sending more work.


## Service pairing

On each client, open Devices and add the service hostname with port 47322. Request access.
On the service host, save `{"operation":"list"}` as a request file and inspect pending identities.
Approve the chosen client with `{"operation":"approve","id":"CLIENT_ID"}`.

```sh
~/Applications/Agent.app/Contents/MacOS/agent --service-device request.json
```

Register execution devices in the other direction through the same service administration command.
Use `{"operation":"find_device","host":"DEVICE_HOSTNAME"}`, then
`{"operation":"request_access","id":"DEVICE_ID"}`. Approve that service identity in the target
app's Devices window. This does not grant direct access between the two execution devices.

Choose the paired service in the chat window. Set defaults in Agent → Conversation settings on each client.
Click device names in the conversation to enable or disable access. Removing a target prevents later submissions;
already dispatched work may have acted. Stop requests cancellation but cannot undo completed input.

Revoking a client blocks new service connections. It does not stop its existing conversations or
revoke the service's separate execution-device grants. To reconnect a revoked client, allow new
requests on the service with `{"operation":"allow_requests","id":"CLIENT_ID"}`, then pair again.

## Recovery

Conversation state is under `service/conversations/<id>`. The catalog and creation defaults are in
`service/conversations.json`. Keep the service Keychain identity and account when changing builds.
Back up the service directory while it is stopped. Client caches are disposable.

After a restart, interrupted work is marked explicitly. Saved tool IDs remain attached to their
original target. Retrieve results does not submit another action. If a target also stopped before
saving its result, the action remains uncertain and must not be repeated blindly.

## Conversation device selection

Messages have a sender device but no preferred tool device. Chat is available from every connected
client regardless of the conversation's enabled devices. The model must select a device ID for each
tool call. The service checks conversation access when dispatching tools and retrieving results.
Device names stay visible in chat, with disabled devices shown in gray. Defaults for new conversations live in Conversation settings.

## Source ownership

`src/agent/service/` runs independently of the desktop UI. Its `chat/` directory owns the loop and
conversation persistence; `chatgpt/` owns credentials, refresh, and inference. The app keeps the
browser callback listener and sends the login result to the service. Shared views and service
messages live in `src/agent/protocol/`. Pairing and execution endpoints live in `src/agent/devices/`.
The shared runtime-directory lookup lives in `src/agent/platform/`.

## Deferred account work

Account switching and an in-app usage display remain deferred.
