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
- [ ] **Next** Compact unified device list, system names, and verified IP or hostname entry.
- [ ] Main chat interface and Rust agent loop using the existing ChatGPT connection and devices.
- [ ] Saved conversation and tool history, Stop, and result retrieval after disconnection.
- [ ] **Later** Model-driven locked-mini proof using those tools.

Deferred login extras include account switching and an in-app usage display. They do not block
pairing. Screenshot input belongs to the agent-loop proof, not the completed login proof.

## Goal

Run one user task through a Rust agent loop in Agent.app, using our existing desktop service on a
locked Mac mini. Keep the GPUI interface small and show each action with its resulting screenshot.

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

## Next agent loop

Use the proven device connections and tools from a Rust loop. Run the first persistent loop on the
mini so it can continue when the laptop disconnects. Keep the design open to loops running on other
devices later. Add task input, action history with screenshots, and Stop.

Defer coordination between simultaneous desktop controllers. Do not add desktop reservations or
scheduling to either milestone.

## Completed ChatGPT login proof

Agent supports one saved account through [Sign in with ChatGPT](https://developers.openai.com/siwc).
Credentials live in macOS Keychain. Refresh runs without a browser. Cancellation, late callbacks,
sign-out, and return to Agent are verified. One model request returned a completed tool call using
subscription access without an API key.

See the [login proof](../../../logs/2026-11-07-13-50-chatgpt-login.md),
[reliability checks](../../../logs/2026-11-08-00-40-login-reliability.md), and
[callback lifecycle](../../../logs/2026-11-08-02-14-persistent-login-callback.md) for details.

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
- Use ChatGPT subscription access. Do not require an API key or silently switch to API billing.
- Keep credentials out of task logs. Report login, quota, and temporary network failures clearly.
- No accessibility-tree queries or app-specific controllers.
- Retain action results and screenshots locally. Do not replay input after an ambiguous failure.
- Require a fresh capture after a service failure. Surface permission and login problems clearly.
- Account for the existing locked-session menu-bar limitation. Do not claim universal app support.
- Leave scheduling, messaging integrations, and multiple concurrent tasks for later.
