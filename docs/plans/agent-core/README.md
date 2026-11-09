# Agent core

## Status

Keep this checklist current whenever work completes or scope changes. Mark an item complete only
when its proof passes. Keep completed implementation history in delivery logs.

- [x] Desktop-service cleanup and app packaging.
- [x] ChatGPT login proof, including refresh, sign-out, cancellation, and return to Agent.
- [x] One subscription-backed model request with a tool-call response.
- [x] Device pairing, authorization, revocation, and connection proof on both Macs.
- [x] Direct host-targeted tools, cancellation, and results available after reconnecting.
- [x] Direct locked-mini proof with screenshots, Calculator, dragging, and long text entry.
- [x] Device discovery, clear one-way access controls, and verification on both Macs.
- [x] Compact unified device list, system names, and verified IP or hostname entry.
- [x] Main chat interface and Rust agent loop using the existing ChatGPT connection and devices.
- [x] Saved conversation and tool history, Stop, and result retrieval after disconnection.
- [x] Model-driven locked-mini proof using those tools.
- [x] Record the sender device on each message so "here" and "this machine" have a clear default.
- [x] Central service with shared conversations, central login, and per-conversation targets.
- [x] Verify client disconnects and service restart without repeating actions.

The [central-service plan](../central-service/README.md) replaces explicit conversation moves.
The move implementation was removed. Conversations stay on the service and are accessible from
every authorized client.

Deferred login extras include account switching and an in-app usage display. They do not block
pairing. The agent-loop proof now includes screenshot input.

## Development policy

Backward compatibility is not required during early development. Remove unused fields and obsolete
paths rather than preserving them for older builds. Preserve user data unless a change requires an
explicit reset. Compatibility becomes a requirement only when explicitly agreed.

## Goal

Run tasks through the central Rust service. Use Agent.app as the client and execution device,
including desktop control on a locked Mac mini. Show actions and their resulting screenshots.

## Completed device pairing

Device pairing and direct remote tools are implemented and verified without inference or an agent
loop. See [direct tool calls](tools.md) and the [delivery log](../../../logs/2026-11-08-03-10-device-tools.md).

- Give each Agent installation a stable device identity and a display name.
- Connect over Tailscale. Network access alone does not authorize Agent requests.
- Pair through a one-time invitation and explicit approval on the receiving device. Show which
  device may control which other device; reverse access is not automatic.
- Keep each private key in Keychain. Remember approved certificates and authenticate connections
  with TLS 1.3 through rustls.
- Show paired devices and allow revocation. Devices report their supported tools, but this
  milestone does not execute those tools remotely.

Prove pairing between the laptop and mini with a harmless authenticated request. Approved devices
must connect successfully. Unpaired and revoked devices must be rejected. Pairing must survive an
app restart.

## Completed device discovery

Both Macs discover Agent through Tailscale. Requests need approval on the target. Incoming access
and outgoing targets have separate controls. Declined requests stay declined until the receiver
allows another request. Manual invitations remain available.

The final installed, notarized build passed discovery, approval, decline, manual pairing, directional
removal, and restart checks. The MacBook can control the locked mini. The mini cannot control the
MacBook. See the [delivery log](../../../logs/2026-11-09-03-45-device-discovery.md).

## Completed direct tools

Direct client calls passed in both directions. ChatGPT login is not required.

- Give shell and computer-use tool calls an explicit `host` that resolves to a paired device.
  Route requests through the authenticated device connection to local tool implementations.
- Prove a shell command and locked-desktop capture over that connection.
- Save task history and command results on the device that owns them. Support reconnecting to
  retrieve status and results without blindly repeating an action.
- Verify cancellation and duplicate requests. Retain action and screenshot pairs for Calculator,
  dragging, and bulk text entry on the locked mini.

## Completed agent loop

The first proof ran the Rust loop in the app process. The central service now runs the loops.
Agent opens a chat window with shared conversations, supported models, and a default target selector.
Setup stays in separate windows under the Agent menu.

The live proof used ChatGPT subscription access from the MacBook. A model-requested shell command
ran on the mini. The model then used screenshots and keyboard input to calculate `9 × 221` in
Calculator while the mini remained locked. The displayed result was `1989`.

Conversation history, tool IDs, and screenshots are now saved on the central service. Stop prevents more submissions
and requests cancellation. After a timeout or restart, Retrieve results reads the original request
without repeating it. Interrupted tasks do not resume automatically.

See the [delivery log](../../../logs/2026-11-09-04-41-agent-loop.md).

## Central service

The service owns conversation storage, ChatGPT login, and agent loops. Device apps provide shared
chat access and execute authorized tools. The mini hosts the first service as a separate process.

Each conversation stores an allowed-target list. MacBook-started conversations initially allow both
MacBook and mini; mini-started conversations allow only mini. Opening a conversation elsewhere does
not change that list. Device creation defaults affect new conversations only.

See the [active plan](../central-service/README.md) and
[delivery log](../../../logs/2026-11-09-15-37-central-service.md).

## Completed ChatGPT login proof

Agent supports one saved account through [Sign in with ChatGPT](https://developers.openai.com/siwc).
Credentials live in macOS Keychain. Refresh runs without a browser. Cancellation, late callbacks,
sign-out, and return to Agent are verified. One model request returned a completed tool call using
subscription access without an API key.

See the [login proof](../../../logs/2026-11-07-13-50-chatgpt-login.md),
[reliability checks](../../../logs/2026-11-08-00-40-login-reliability.md), and
[callback lifecycle](../../../logs/2026-11-08-02-14-persistent-login-callback.md) for details.

## Boundaries

- Keep the Swift service responsible for capture and input. Keep task state and model interaction
  in Rust. Use GPUI for login, task entry, progress, and stopping a task.
- Use ChatGPT subscription access. Do not require an API key or silently switch to API billing.
- Keep credentials out of task logs. Report login, quota, and temporary network failures clearly.
- No accessibility-tree queries or app-specific controllers.
- Retain action results and screenshots locally. Do not replay input after an ambiguous failure.
- Require a fresh capture after a service failure. Surface permission and login problems clearly.
- Account for the existing locked-session menu-bar limitation. Do not claim universal app support.
- Leave scheduling, messaging integrations, and coordination between separate conversations for later.
