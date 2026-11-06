# Computer-use retry

## Context

Noah requested another Calculator test through computer use on both the MacBook and Mac mini.
He then requested a fresh mini screenshot and PyAutoGUI proof.

## Decisions

- Noah requested that we continue until desktop actions work. Next diagnose global event
  delivery with passive listeners restricted to our tagged synthetic mouse events. Do not
  record user input. Test an explicit combined-session event source.
- Noah then explicitly approved testing a small generic coordinate-to-window router.
  Process-directed event delivery is allowed for that router. App-specific behavior and
  accessibility-tree use remain excluded.
- Use direct SSH at Noah's request. It now works without another interactive Tailscale check.
  This replaces remote Codex chats for the current proof.
- Test vision and global mouse input while locked. Do not use an accessibility tree,
  process-targeted input, automatic unlock, or a full assistant implementation.
- Treat a connection failure separately from a Calculator failure.

## Changes

- Saved the successful bounded experiment under `probes/coordinate-router/` with a reproduction
  guide and explicit limitations. Updated the root README to distinguish proven routed clicks
  and typing from failed global input and unproven full-desktop behavior.
- At Noah's explicit request, created "Resolve Mac mini locked computer use" directly on the
  mini: `01a1a0de-0052-7a03-b7ed-8c1028306dcc`. Supplied prior evidence and instructed it to
  complete independent permitted work without reissuing the pending escalated diagnostic or
  bypassing its approval. The creation tool exposes no approval-policy parameter.
- Confirmed the new chat started running with no waitingOnApproval flag.
- Ran Calculator through this chat's `cua_repl` on the MacBook.
- Attempted the existing SSH connection to `joah@joahs-mac-mini` outside the sandbox.

## Verification

- The approved generic coordinate router succeeded while locked. Its corrected `NSEvent`
  construction closed the visible completed trace window at `(565, 589)`. It then clicked
  Activity Monitor's exposed search field at `(1640, 142)`, typed `731103`, and clicked the
  clear button at `(1808, 142)`. Repeated with `86204`, then cleared again. Both typed
  results and both clears were verified in fresh images. The caller supplied only visual coordinates and
  key codes. Destination selection used window rectangles, with no app-name branching or AX tree.
- The working event uses the window number, compensated window-relative coordinates, and
  `NSProcessInfo.systemUptime()` before `CGEventPostToPid`. On this Python environment,
  `time.monotonic()` starts near zero at process launch. Earlier NSEvent trials used that
  unsuitable timestamp. The successful trial corrected the timestamp and used the compensated
  coordinates; it does not isolate which change is independently necessary.
- Generic keyboard delivery uses the last mouse-selected process. The typed markers appeared
  in the visually selected field. An earlier Terminal Command-F trial had no visible effect.
- The corrected title drag still did not move the window. Do not claim complete desktop control,
  reliable multi-window keyboard focus, or plain PyAutoGUI success from these results.
- Session lock checks ran before every mouse/key event and capture. Separate IORegistry checks
  after typing and cleanup showed both lock flags true. The search was restored, the bounded
  router stopped, and no `coordinate_router` process remained. No permissions were changed.
- The saved router's parsed syntax tree matches the remotely executed v4 source. Only formatting
  changed. Both saved Python files passed local syntax compilation without executing GUI actions.
- At 20:53 JST, passive mouse listeners successfully observed our tagged drag events at
  the session and annotated-session event taps. All four events targeted PID 453 and
  annotated window 1358. `ps -p 453 -o comm=` identified Apple's `loginwindow` process.
  Thus this explicit combined-session-source test delivered input to the lock-screen
  process, not the pictured app desktop. Fresh before/after images showed no window move.
  Input posting, listening, and Accessibility trust checks were all true. The lock flag
  remained true. This establishes routing for this test, not a universal impossibility.
- Asked whether a generic coordinate-to-window router would satisfy Noah's design constraint.
  It would use window geometry and process delivery, without app-specific behavior or an
  accessibility tree. Noah approved this test; proceed with process-directed input chosen
  only from generic window geometry and visual coordinates.
- On Nov 3 at 20:40 JST, direct SSH to `joah@joahs-mac-mini` succeeded. An approved
  outside-sandbox SSH command launched the existing Terminal app with a small proof script.
  No remote Codex chat or approval bypass was used.
- The Terminal-launched Python process had a valid graphical session. Fresh PyAutoGUI and
  native display captures both returned 1920×1080 wallpaper, without the app desktop.
  Both lock flags were true before and after capture.
- A fresh native image composed from visible layer-zero windows showed the app desktop,
  including each newly launched proof Terminal window. It is a window composite, not a
  faithful full-display screenshot. Menu bar and Dock coverage remain unproven.
- At 20:45 JST, one PyAutoGUI click at the visually selected exposed Calculator zero button
  coordinate `(586, 870)` returned successfully but produced no visible app change. The
  session stayed locked. This does not prove successful desktop input.
- At 20:47 JST, tested two more global event routes: `kCGSessionEventTap` and
  `kCGAnnotatedSessionEventTap`. Each attempted a visible Terminal-title drag from
  `(780, 531)` to `(1000, 371)`. Neither moved the window in the fresh after image.
  Lock checks passed before each event and before each after capture. No PID, app command,
  or accessibility tree was used. PyAutoGUI uses `kCGHIDEventTap` for its mouse events.
- The same graphical launch path reported `post_event_allowed=true`,
  `screen_capture_allowed=true`, and `accessibility_trusted=true`. Its console and login
  flags were true, uid was 501, audit session was 100011, and locked was true. This rules out
  absent reported input permission or absent graphical session as explanations for these
  fresh failures. It does not establish which macOS component dropped the events.
- Screen Sharing is already enabled. A read-only protocol greeting on mini localhost port
  5900 returned `RFB 003.889` and security types `[30, 33, 36, 35]`. No authentication was
  attempted and no settings were changed. This is a concrete next route, not proof that
  its desktop meets the lock requirement.
- Evidence and bounded proof scripts are in `.scratch/2026-11-03-20-40-desktop-proof/`
  locally and under `/Users/joah/.scratch/` on the mini. Before/after images were inspected.
  No keys were typed because no test had established desktop keyboard focus.

- Further permitted mini diagnosis ruled out a different audit session: probe and logged-in
  console both use audit session100011 and uid501; launchd reports Aqua. `CGSession=None` is
  therefore not evidence that the probe belongs to a different login session.
- The probe reports `CODEX_SANDBOX=seatbelt`. Its installed CLI0.166.0 and app-bundled
  CLI0.166.0-alpha.5.1 contain default-deny policy text with no explicit WindowServer or
  LaunchServices allowance in the extracted mach-lookup blocks. This supports sandbox isolation
  as a hypothesis, but static embedded policy is not a dump of effective runtime policy.
- No verified sandbox_check declaration/constants were available in the bounded installed-source
  search. No private ABI was guessed, no policy changed, and no GUI proof was claimed.
- Requested a fresh native desktop-composite and coordinate-action proof on the mini. At19:33 JST,
  its remote chat confirmed lock state, but current Python had no CG session, capture preflight
  and Accessibility trust were false, and no relevant graphical apps were visible. Terminal's
  executable exists despite ordinary LaunchServices returning -10827. No new image or input
  resulted. This execution context cannot reproduce the earlier Terminal-session proof as-is.
- Noah requested fresh proof of plain PyAutoGUI while locked. On Nov3 at19:19 JST, the new mini
  chat ran `pyautogui.screenshot()` using the existing venv. Both lock flags stayed Yes, but the
  remote Python context had no CG session. Capture failed with "could not create image from
  display 0", then `PIL.UnidentifiedImageError`; the temporary image was zero bytes.
- Tried the existing graphical Terminal launch setup without escalation. App-name lookup failed.
  After confirming `/System/Applications/Utilities/Terminal.app` exists, ordinary full-path
  LaunchServices launch failed with `NSOSStatusErrorDomain Code=-10827 kLSNoExecutableErr`.
  No graphical test output appeared and no global keys/clicks were sent. This does not prove
  Terminal is damaged, nor does it establish that PyAutoGUI is impossible in a graphical session.
  Fresh evidence is under `/Users/joah/.scratch/2026-11-03-pyautogui-locked-proof/`.
- Plain PyAutoGUI on the locked mini remains unproven. The earlier successful calculation used
  native targeted input and window capture and must not be described as a PyAutoGUI success.
- The new remote chat completed its independent checks without an approval wait. It visually
  verified the saved locked Calculator result, confirmed the mini remains locked, and established
  that the mini's bundled plugin hash matches the MacBook's bundled plugin hash.
- Its installer inspection identifies `CodexComputerUseAuthorizationPlugin:allow` under
  `system.login.screensaver` as the installation target. Runtime registration remains unknown:
  `security authorizationdb read system.login.screensaver` returned 255 with empty output.
  No protected-database workaround was attempted. Sandboxed signature checks also failed for
  `/bin/ls`, so plugin signature failures are inconclusive. No repair or fresh UI proof was made.
- Remote findings are saved at `/Users/joah/.scratch/2026-11-03-mini-local-diagnosis/findings.txt`.
- Noah declined further manual approval participation. Sent the mini chat an instruction to cancel
  its pending diagnostic request if supported, without approving it or bypassing restrictions,
  and continue with accessible evidence. The message was accepted but the chat status did not change.
- Retrieved the available diagnostic output at the reader's 20,000-character limit. The retained
  log excerpt contains repeated media errors; unlock-specific records are not available in that
  excerpt. This does not establish that the full log lacks them.
- Both locally inspected authorization-plugin binaries declare minimum macOS 14.4 and SDK 26.1.
  Minimum-OS metadata therefore does not itself explain the mini's failure on macOS 15.7.5.
- The local service binary contains separate messages for automatic-unlock failure and automatic
  unlock paused by physical input. The mini returned the former. String inspection alone cannot
  identify the failing runtime branch or a repair.
- Continued the existing mini chat with a focused diagnostic follow-up after Noah instructed us
  to proceed. The send succeeded, but the chat remained waitingOnApproval and did not process
  the follow-up during the status checks. Native computer use refused inspection of the Codex
  app itself, so that route cannot expose the pending prompt.
- Verified locally that the working MacBook's installed authorization plugin is version 0.1.0,
  build 829. Its executable SHA-256 is
  `204f129aed3aa1248968cb9efc380e208b6a6b2eb938b6fa0e30e6469e8b0766`.
  The bundled executable SHA-256 is
  `9b537c0513f2089f00d4309315ae1ea0cb63f008c85cd2ed70517215595ff626`.
  Thus the installed and bundled MacBook plugins differ. The mini reported installed and bundled
  plugin version 26.1022.1001641. This is a diagnostic lead, not proof of causation.
- Discovered remote project `13311e41-aacd-498c-88d1-269402aaf466` on host
  `remote-control:env_e_6ae5b26bc4d88320bb133219ae8b4d2a`. Read its existing chat
  `01a1a036-8fc1-7270-bd4c-d56e8a8a7d8e`, titled "Fresh Mac mini locked Calculator test".
- That chat tested the mini directly at 2026-11-03 07:01:39 UTC. Both lock flags were Yes
  before and after. `cua.getApp("Calculator")` failed in about 0.33 seconds with:
  "The Mac is locked and automatic unlock could not unlock it. Ask the user to unlock the Mac
  manually before continuing." This is independent of the SSH authentication problem.
- Its diagnosis found macOS 15.7.5 build 24G624 and app build 23880 on the mini, versus
  macOS 26.6.2 build 25G83 and app build 22943 on the MacBook. Both bundled computer-use
  services and Guardians are version 26.1022.1001641. Differences do not establish causation.
- Local host identifies itself as `noah-macbook-pro-m5.local`.
- The shipped tool entered `94 × 23`. Calculator's returned accessibility state showed the
  expression and result `2,162`.
- The mini connection returned `Tailscale SSH requires an additional check` before executing
  the remote command. No Calculator retry ran on the mini.
- A fresh SSH attempt for the Python proof reached the same Tailscale check. It did not execute
  `hostname` or any UI action. The prior locked success used `CGEventPostToPid` and window capture,
  not plain PyAutoGUI. Do not describe it as a successful locked PyAutoGUI test.

## Follow-ups

- The small locked click-and-type proof is complete. Before building the assistant, assess the
  remaining limits: window dragging, menus, browser content, keyboard focus across windows,
  full-display capture, and operation after sleep/relaunch. Current evidence does not justify
  treating this as a general desktop controller yet.
