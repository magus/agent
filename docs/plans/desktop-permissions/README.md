# Desktop permissions

## Status

Permission readiness and revocation checks pass. The controlled mini tests disable each grant
separately, verify rejection and working shell access, then restore both and verify locked capture
and input. The separate macOS capture confirmation can still recur; this work does not remove it.

- [x] Report computer-use readiness from silent macOS permission checks.
- [x] Reject unavailable computer actions before dispatch without triggering permission dialogs.
- [x] Keep permission setup explicit. Ordinary startup and discovery use silent checks.
- [x] Verify disabled, granted, and restored permissions on the mini.
- [ ] Verify the extra capture confirmation through setup, restart, and unlock. Do not treat a
  successful screenshot as proof that no confirmation remains pending.

## Capture confirmation

- [ ] Verify declining the macOS capture dialog, then retrying through explicit setup.
- [ ] Verify the user is told when capture becomes unavailable during unattended work, without
  replaying input that may already have run.

Setup must run a real capture through the bundled helper while the user is present. Show the image
for inspection. If macOS asks for approval, the user must click Allow. After setup, verify locked
capture and input. Setup capture and ordinary computer actions both run locked or unlocked.

There is no established public check for acceptance of the extra capture confirmation. Screen
Recording permission and a successful screenshot do not guarantee that the dialog cannot recur.
Do not treat a saved successful preview as permanent approval. If capture fails, retain the error,
stop using the previous image, and obtain a fresh capture after the user resolves the issue.

## Scope

Devices advertise computer use only when silent helper checks report both required permissions.
Discovery caches the result for two seconds. The helper checks again before execution so revocation
takes effect. A running helper can retain a stale permission result after revocation. Rust therefore
also runs a fresh helper permission probe before dispatching each computer action. A failed or
unavailable probe rejects the action before it reaches the running helper.
Keep shell access independent of desktop permissions.

Startup, discovery, chat, and rejected tool calls must not trigger permission dialogs.
Users should choose desktop setup before Agent requests access.

## Verification

The controlled mini test with Accessibility disabled found and fixed stale permission results in
the running helper. The updated app advertises shell only, still executes shell commands, and rejects
capture before dispatch. Screen Recording-off also passes after macOS's Quit and Reopen action.
Shell still works and capture is rejected. After restoring both grants, locked capture and input
pass; Calculator shows `9 × 221 = 1989`. Noah reported no unexpected dialogs during the tests.

Code review confirms the native setup buttons only open the relevant System Settings pane.
The helper uses `AXIsProcessTrusted` and `CGPreflightScreenCaptureAccess` before desktop input
and capture. Discovery uses the same silent checks and advertises shell separately. Use the approved Agent identity for
runtime checks; do not create another app or reset the user's grants to manufacture a test case.

Check startup, device discovery, and tool calls with permissions missing, granted, and revoked.
Confirm that ordinary chat and shell commands still work without desktop permissions.
Confirm that explicit setup can obtain permissions and enable computer use.

The unexpected Accessibility dialog reported on November 10 remains unexplained. Do not assume
this readiness bug caused it. See the [investigation log](../../../logs/2026-11-10-23-30-accessibility-prompt.md).
