# Agent core

Desktop-service cleanup and packaging are complete. ChatGPT login and a single model-request
proof passed on the local Mac. The agent loop is not implemented.

The next milestone is device pairing and authorization. Remote tool execution and disconnect
handling follow later, as part of the simple agent loop.

## Goal

Run one user task through a Rust agent loop in Agent.app, using our existing desktop service on a
locked Mac mini. Keep the GPUI interface small and show each action with its resulting screenshot.

## Next milestone

Make explicit device connections a standalone Agent feature, without adding the agent loop or
remote shell and computer-use tools yet.

- Give each Agent installation a stable device identity and a display name.
- Connect over Tailscale. Network access alone does not authorize Agent requests.
- Pair through a one-time invitation and explicit approval on the receiving device. Show which
  device may control which other device; reverse access is not automatic.
- Keep each private key local. Remember approved public keys and authenticate connections using
  an established protocol. Select the protocol before implementation.
- Show paired devices and allow revocation. Devices report their supported tools, but this
  milestone does not execute those tools remotely.

Prove pairing between the laptop and mini with a harmless authenticated request. Approved devices
must connect successfully. Unpaired and revoked devices must be rejected. Pairing must survive an
app restart.

## Later agent loop and tools

Build remote execution alongside the simple task loop, after device pairing is proven.

- Run the first persistent loop on the mini so it can continue when the laptop disconnects.
  Keep the design open to loops running on other devices later.
- Give shell and computer-use tool calls an explicit `host` that resolves to a paired device.
  Route requests through the authenticated device connection to local tool implementations.
- Prove a shell command and locked-desktop capture over that connection.
- Save task history and command results on the device that owns them. Support reconnecting to
  retrieve status and results without blindly repeating an action.
- Add task input, action history with screenshots, and Stop.

Defer coordination between simultaneous desktop controllers. Do not add desktop reservations or
scheduling to either milestone.

## ChatGPT login

Include a ChatGPT login flow and use eligible ChatGPT Pro subscription quota. Do not require an
OpenAI API key or silently switch to API billing. Show the signed-in account, available usage status,
and a clear recovery path for expired login or exhausted quota. Keep credentials out of task logs.

Login and one subscription-backed model request are proven. Test screenshot input later, while
building the task loop and exposing computer-use tools to the model.

The current proof uses [Sign in with ChatGPT](https://developers.openai.com/siwc), with dynamic
registration for Agent, a loopback callback, identity validation, and macOS Keychain storage.
The dcssweb proof informed callback, refresh, and streaming behavior; its older Codex client ID
and backend endpoint are not used here. The first test sends only an arithmetic prompt and requests
a tool-call result. Screenshot input will be verified as part of the task-loop work.

This first proof supports one saved account. Live cancellation, background token refresh, local
sign-out, and remote revocation passed. Login and sign-out persisted across app restarts.
The model catalog selected `gpt-6.1-sol`, which returned `1989` through a completed tool call
using ChatGPT login with no API key. Automatic focus is confirmed. Activation follows the state
update and explicitly requests a redraw. A timed live run rendered the enabled button 7 ms after
the focus request; the user confirmed it was ready when Agent appeared. Window activation was
reported about 4.8 seconds after the request, so this does not establish instant activation.
Account switching and an in-app usage display remain later work;
the proof links to ChatGPT Settings → Usage.

Refresh uses the saved refresh token in the background and saves replacement tokens to Keychain.
It must not open a browser. Ask for sign-in only when the saved session can no longer be renewed;
temporary network failures remain retryable. Regression tests cover cancellation and the distinction
between unusable tokens and temporary errors. Pending login has no app-imposed deadline. The callback
server starts on first login and stays open until Agent quits. Late callbacks show the attempt's
result without accepting credentials again. The pending login button stays clickable; hover shows
"Cancel login", and clicking cancels. Per-attempt timing hooks record login stages, focus, and
rendering without credentials.

## Agent-loop proof

1. Sign in from Agent and confirm one screenshot-based model request uses subscription access.
2. Accept one task and capture the locked desktop through the Swift service.
3. Let the model choose a mouse, keyboard, drag, or bulk-text action using vision only.
4. Execute through the service and show the returned screenshot alongside the action.
5. Continue until the task finishes, the user stops it, or a concrete failure needs attention.

Use Calculator for the first complete task. Then verify dragging and a long text paste in another
app. The loop runs on the mini and must not depend on a connected MacBook or an unlocked desktop.

## Boundaries

- Keep the Swift service responsible for capture and input. Keep task state and model interaction
  in Rust. Use GPUI for login, task entry, progress, and stopping a task.
- No accessibility-tree queries or app-specific controllers.
- Retain action results and screenshots locally. Do not replay input after an ambiguous failure.
- Require a fresh capture after a service failure. Surface permission and login problems clearly.
- Account for the existing locked-session menu-bar limitation. Do not claim universal app support.
- Leave scheduling, messaging integrations, and multiple concurrent tasks for later.
