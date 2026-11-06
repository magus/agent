# Native desktop agent

## Context

Build and prove a small local desktop-control service while Joah's Mac mini remains locked.
Use screenshots, mouse coordinates, and keys. Generic window geometry routing is allowed;
accessibility trees and app-specific controller branches are not. Long text must use bulk insertion.
The service must survive a restart and operate without a live MacBook or Codex connection.

## Decisions

- Use Swift and installed macOS frameworks, with no package dependencies or Python runtime.
- Check both `CGSSessionScreenIsLocked` and IOKit `IOConsoleLocked` before and after each operation.
  Drag and key loops check during input too. Never unlock the mini or change security settings.
- Treat successful dispatch separately from observed UI effects. Never replay input automatically
  after an error because it might already have acted. Capture again after animations when needed.
- Preserve all available clipboard representations and verify restoration by hash. Do not overwrite
  a clipboard that another process changed during paste.
- Keep the existing Terminal permission context. A separately authorized agent app identity and
  login item are outside this proof. Preserve the existing message bridge and reply routing.
- Keep the controller generic. Native event routing uses window geometry and pixel alpha, not app
  names, accessibility nodes, foreign-process injection, or window-position setters.

## Changes

- Added `Sources/DesktopAgent` with capture, click, drag, scroll, keys, and bulk paste. It accepts
  stdin, local job files, or a same-user Unix socket. There is no TCP listener or model loop.
- Added build, install, and supervisor scripts in `scripts/`. The mini's running installation is
  `/Users/joah/.local/share/agent-desktop`. Its launcher is minimized; the old scratch service stopped.
  `Sources/DesktopAgent/README.md` documents installation, requests, and limits.
- Capture composes windows below the screen-saver layer at their desktop positions. Layer ordering
  and associated-window grouping preserve visible sheets. This is a full-desktop window composite,
  not an image of the physical lock screen.
- Added expected-window guards and a one-pixel alpha check. The latter prevents transparent
  full-screen overlay regions from intercepting app clicks. Keyboard input retains mouse selection.
- Fixed wheel routing to use window-local explicit coordinates. Mouse events use desktop coordinates.
- Added modifier flags and up to eight drag waypoints. The working native route maintains shared
  button state and cursor motion. Once geometry confirms native window movement without resizing,
  it stops duplicate app-directed motion. Content dragging retains app-directed events.
- Removed alternate mouse encodings and temporary motion/trace switches after proving the default.
- Added a generic menu-bar route to the selected user app. It uses a windowless event with the real
  menu-bar ID in pointer fields. It does not post input to WindowServer.

## Verification

- Swift compiled locally and on the mini's macOS 15.7.5 arm64. Input and capture grants were already
  available. Both lock indicators stayed true throughout successful tests; no new grant was needed.
- Fresh desktop images measured 1920x1080. Real input effects passed in Terminal, TextEdit,
  Activity Monitor, and Calculator. No app-specific branches or accessibility queries are in source.
- Calculator coordinate clicks entered `73 × 19`, and a later capture visibly showed `1387`.
  The final capture was at Unix time 1793722136.646. Supporting local image:
  `.scratch/2026-11-03-21-47-drag-and-paste/calculator-73-times-19.jpg`.
- Terminal accepted a new shell, bulk command text, and Return. A harmless command printed 100
  numbered lines with Japanese and emoji. Native scrolling revealed earlier output. A standard
  AppKit scroll view also moved from line 1 to line 5 after the wheel fix.
- Activity Monitor's search field visibly received `desktop-agent`. Command-A and Delete removed
  the temporary filter. Clipboard restoration passed.
- Two windows in one process received distinct ALPHA/BRAVO markers correctly. Removing a redundant
  activation before paste fixed a lost-key-window problem.
- TextEdit coordinate selection replaced `beta` with `BETA`; Command-S saved exactly
  `Alpha BETA gamma\nSecond line\n`. SHA-256:
  `93f4824100062afdf25c822f68a531e6cb08d2e00f5cb63b624825169767e638`.
  Repeating selection after the final drag-handoff change replaced `BETA` with `beta` exactly.
  The resulting SHA-256 was
  `10ac8feee510397ac25e801ef5bec385094a90c9666eaf5f41fc69e9872e947b`.
- Bulk paste saved 82,000 characters, 2,000 lines, and exactly 108,000 UTF-8 bytes. The payload
  included Japanese, emoji, accented and combining characters, and tabs. SHA-256:
  `dc17d41948a6ea79d648dd351da8aaf2424248e0de3fba6d0de00e7122da987c`.
  Clipboard restoration matched by hash. Input uses a single paste, not delayed per-character keys.
- The final installed-service job ran after its SSH launcher exited at Unix time 1793721573.
  Local requests ran from 1793721593.616 to 1793721596.510: click, bulk paste, save, and scroll.
  It saved the exact 108,000-byte hash above. Paste dispatch took 1.154 seconds, including the fixed
  one-second clipboard wait. All responses passed lock checks; stderr was empty.
- A separate delayed native job completed with all network operations denied to its process.
  SSH exited at 1793719020; actions ran from 1793719040.405 to 1793719163.524. Exact saved bytes,
  clipboard restoration, and both lock checks passed. That strict test profile added about 30
  seconds per capture, so it is not applied to the service. These tests establish independence from
  a live MacBook connection. Physical MacBook network disconnection was not performed.
- Controlled child restarts recovered the socket under the surviving supervisor without another
  permission prompt. Runtime mode is 0700 and socket mode is 0600. Stale socket recovery passed.
- Wrong expected-window IDs rejected input before posting. A delayed job stopped when its launcher
  covered the target; retry after observing and clearing the obstruction passed. Cleanup guards
  likewise stopped before clicking an occluded window. Only identified proof windows were closed
  or minimized. The live launcher remains minimized and running.

Window dragging now works in unmodified apps:

- Middle-title input did not start standard dragging. Passive diagnostics showed a title text field
  with `allowWindowDragging=false`. One pixel below an exposed top edge, a horizontal move of about
  12 pixels starts native movement. Continue to the destination after that. A vertical start resizes.
- Earlier paths stopped short because app-directed motion competed with native movement. Traces
  confirmed cursor motion and shared session button state. Geometry-based handoff fixed that issue.
- With the final default, Terminal window 1511 moved from X=172,Y=143 to X=272,Y=243, then exactly
  back to X=172,Y=143, retaining its 570x371 size. TextEdit 1458 moved from X=313,Y=467 to X=50,Y=100,
  retaining its 646x418 size. Content selection still passed with the same default.
- Direct native drag requests, focus calls, Command-drag, cursor warping, and drag-session transfer
  did not solve Terminal movement. Read-only inspection ruled out missing mapping and wrapper
  options. A custom fixture's earlier success alone was insufficient; real-app tests supersede it.

Menu and activation results remain narrower:

- Terminal's Shell menu opened. Arrow keys selected New Command; Return opened its blank dialog.
  A coordinate Cancel click dismissed it. A direct popup-item click remains unverified.
- Selecting TextEdit does not replace the captured Terminal menu bar. A public activation request
  completed without error but left TextEdit inactive. General app activation is not established.
- A later Command-N test appeared unsuccessful in immediate captures, but a fresh shell window
  1768 subsequently appeared behind TextEdit. Do not infer failure or replay from one early frame.
- A hidden Terminal Save sheet became visible after associated-window grouping. Command-period
  dismissed it; Escape and Cancel clicks did not. No file was saved in that test.

Initial compiler errors and one stale-PID restart attempt were corrected. Failed hidden-symbol
lookups stayed in scratch diagnostics. No process ownership or security setting was changed.
Local Swift build, shell syntax checks, and whitespace checks passed after documentation cleanup.

## Follow-ups

- The required desktop-control proof is complete. General menu activation, right-button input,
  double-clicks, Dock launching, cross-app drag-and-drop, protected content, and multiple displays
  remain outside verified coverage. Standard window movement needs the documented top-edge gesture.
- The service uses undocumented SkyLight and Carbon calls plus an exported capture function removed
  from the current Swift SDK interface. Retest after OS changes. This build deliberately rejects
  unlocked operation and still needs a logged-in desktop after reboot.
- A model loop, messaging integration, separate app permission identity, and automatic login startup
  are not implemented. The existing bridge was left unchanged.
