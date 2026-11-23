# Desktop service

The native app's `src/clients/apple/macos/DesktopService.swift` starts and supervises the helper in `swift/`.
The Swift helper has no package dependencies. It runs locally on the mini and
accepts screenshots, mouse coordinates, keys, and bulk text. It contains no app-specific controller
branches or accessibility-tree queries. It is not a model or messaging loop.

For the signed native app containing this helper, see
[native builds](../../packaging/apple/README.md). Run the service through that app so it uses
Agent's permission identity.

Agent's desktop permission window shows permission status and a capture preview. Remaining checks
are in the [desktop permission plan](../../docs/plans/desktop-permissions/README.md).

## Install and run

Build and launch Agent.app using the [native build instructions](../../packaging/apple/README.md).
The app starts this helper in the logged-in graphical session.
Grant Accessibility and Screen & System Audio Recording to Agent. After reboot, login is still required to create
that desktop. Launch at login is not implemented yet.

The runtime directory is `~/Library/Application Support/Agent/`, with mode 0700;
`agent.sock` is mode 0600. The Swift helper has no TCP listener. Agent's Rust device service can expose
these tools to explicitly paired devices over TLS. Other processes of the same user can send local
requests. The socket is not a sandbox against that user's apps.

The native app restarts a failed helper, starting after two seconds and backing off to a minute
while it keeps failing at once. Quit Agent to stop its helper. The helper exits when its app exits,
even after a crash or forced quit, and the next launch stops any leftover helper from the same
bundle whose app is gone. The central
Rust service runs independently and continues when the desktop app quits.

Closing the setup window leaves the service running. Reopen it through
**Agent → Desktop Permissions…**. Reopening Agent shows the chat window. **Quit Agent** stops both processes.

Actions run whether the Mac is locked or unlocked. Each result reports `locked` from the session lock
flag and `IOConsoleLocked`. The service never unlocks the Mac. While unlocked, input moves the user's
pointer and focus. An earlier version accepted actions only while locked; that restriction was removed.

## Requests

Write a JSON request to a private file, then send it:

```sh
app_path="$HOME/Applications/Agent.app"
runtime_dir="$HOME/Library/Application Support/Agent"
"$app_path/Contents/MacOS/desktop_service" "$runtime_dir" --send request.json
```

Each response includes a fresh image path and timing. `ok: true` means dispatch and capture succeeded.
It does not prove the intended UI effect. Inspect the image. An error may occur after input already
acted, so never retry blindly. Animations can outlast the immediate capture; request another frame.

Capture once before the first input after startup or a failed action. The service checks permission
grants and obtains another capture before dispatching input. A failure clears the selected window;
after resolving it, capture again and click the intended window before sending keys.

```json
{"id":"observe","operation":"capture"}
```

Capture returns logical screen dimensions, image dimensions, and window IDs, bounds, and layers.
The image composes windows below the screen-saver layer in their desktop positions. It includes
sheets and the menu bar. It is not a raw image of the physical lock screen. Protected content,
multiple displays, and mixed display scaling are unverified.

```json
{"id":"click","operation":"click","x":350,"y":600,"expected_window_id":1458}
```

Choose logical coordinates from a fresh image, not these examples. `expected_window_id` rejects a
changed target before input. The router checks pixel alpha so transparent overlay regions do not
intercept clicks. Mouse input uses the left button and can move the shared cursor. `flags` accepts a
CoreGraphics modifier bitmask.

Keyboard input goes to the last mouse-selected app process. Clicking its intended window establishes
key focus. Menu popups preserve the previous app selection. Virtual key codes identify physical Mac
keys; for example, Command-S is code 1 with modifier flag 1048576.

```json
{"operation":"keys","key_list":[{"code":1,"flags":1048576}]}
```

Use bulk paste for text. It snapshots all available clipboard representations, posts Paste, waits
one second for consumption, then restores the clipboard if no other process changed it. Restoration
is checked by hash. Very slow apps still require visual verification. The number of key events does
not grow with text length, though encoding and copying more bytes still take work.

```json
{"operation":"paste","text":"A multiline block\nwith Unicode 日本語 🙂\n","replace":true}
```

`replace: true` sends Select All first. `false` replaces the selection or inserts at the caret.
Text is limited to 4 MiB of UTF-8. Requests are limited to 8 MiB. Use at most 100 keys per key request.

```json
{"operation":"scroll","x":350,"y":600,"delta_y":10,"expected_window_id":1458}
```

Scroll units are lines. Positive values reveal earlier content in the tested apps. A scroll request
supports -100 through 100 lines.

```json
{"operation":"drag","x":357,"y":502,"end_x":384,"end_y":502,"expected_window_id":1458}
```

Standard window movement on this macOS 15 mini needs a specific gesture: start one logical pixel
below an exposed top edge, move horizontally about 12 pixels, then continue to the destination.
A vertical start selects resizing. Middle-title dragging remains unreliable.

```json
{"operation":"drag","x":559,"y":244,"waypoint_list":[{"x":571,"y":244}],"end_x":459,"end_y":144,"expected_window_id":1511}
```

`waypoint_list` accepts at most eight intermediate points. When geometry shows the window has begun
moving without resizing, the router stops duplicate app-directed motion and lets macOS finish the
drag. Content drags retain their app-directed events. No window-position setter is used. Observe the
final position because window snapping can affect it.

`--stdio` accepts JSON lines. `--job FILE` runs a local `actions` array with an optional
`start_delay_seconds` of 0 through 3600. These modes need the same GUI permission context as the
service. Static jobs cannot adapt to new windows; use current coordinates and expected-window guards.
A separate local controller can use `--send` without a GUI connection or an active SSH connection.

## Limits

- Cross-app menu-bar activation remains limited. Selecting a window can leave another app's
  menus visible. Direct popup-item clicks remain unverified.
- App activation and window raising are not universal. New windows may appear late or behind
  another window. Inspect effects before dependent input.
- Right-button input, double-clicks, Dock launching, and general cross-app drag-and-drop are unverified.

The route uses undocumented SkyLight and Carbon functions. Capture uses an exported CoreGraphics
function removed from the current Swift SDK interface. OS upgrades require retesting. Event
signatures were checked against the [CUA input source](https://github.com/trycua/cua/tree/main/libs/cua-driver/rust/crates/platform-macos/src/input).

Exact evidence, corrections, and remaining work are in the
[native service log](../../logs/2026-11-03-22-07-native-desktop-agent.md).
