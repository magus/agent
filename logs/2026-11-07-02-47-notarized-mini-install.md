# notarized-mini-install

## Context

The user requested installing the actual notarized Agent.app on the mini and proving locked control.

## Decisions

- Install the exact `dist/Agent.app` with final identifier `com.iamnoah.agent`.
- Stop and preserve the older proof bundle before replacing it. Keep its runtime data.
- Verify signing, stapling, Gatekeeper, and permissions on the target before input tests.

## Changes

- Installed the exact notarized bundle at `/Users/joah/Applications/Agent.app`.
- Stopped and unregistered the earlier proof. Preserved it at
  `/Users/joah/.scratch/2026-11-07-02-47-notarized-mini-install/Agent-proof.app.disabled`.
- The final app uses `~/Library/Application Support/Agent`; old proof runtime data remains intact.

## Verification

- On the mini, strict nested signature verification, stapled-ticket validation, and Gatekeeper
  assessment passed. Gatekeeper reported `Notarized Developer ID`.
- Both installed executable SHA-256 hashes match the local notarized build exactly:
  Rust `fecb26210f00ff4f54db2a352bd2a55f2d5ae5418267bb9e52777c301efcf5c9`;
  Swift `509a0d226709c7989fdb1194bbf73451a4fffbe44b53e98d4d30b1d4dd2ca08b`.
- Normal app launch on macOS 15 succeeded. Its read-only diagnostic reported Accessibility=false
  and Screen Recording=false for the final identity, as expected before initial grants.
- The user could not remove an old Agent row from Accessibility. TCC logs repeatedly failed to
  resolve `dev.noah.agent.proof`, while LaunchServices correctly registered `com.iamnoah.agent`.
- The targeted `tccutil reset Accessibility dev.noah.agent.proof` initially failed with -10814
  because the old bundle was unregistered. Temporarily registered the preserved proof without
  launching it, reran the targeted reset successfully, then unregistered and archived it again.
  No other app's grants or the final identity's grants were reset.
- After the user's grants, the installed final app reported Accessibility=true and
  Screen Recording=true. The first capture request was rejected by the unlocked-session guard,
  before any input. The user then locked the mini.
- The final installed app passed the locked Calculator test. Captured the 1920×1080 desktop,
  clicked Calculator's AC button at (539, 676), then sent keys for `9 × 221 =`.
  Inspected each screenshot: the old result changed to `0`, then to `9×221` and `1989`.
  All three responses reported `ok: true` and `locked: true`.
- Requests used the final bundle's helper and `~/Library/Application Support/Agent/agent.sock`.
  This verifies the notarized app with its own grants, rather than the older Terminal-run service.
  Evidence is in `.scratch/2026-11-07-02-47-notarized-mini-install/` as `before`, `clear`, and
  `result` JSON/JPEG pairs. The final app remains running.

## Follow-ups

- The planned capture-approval setup flow remains separate work. This successful test does not
  establish that macOS will never repeat its direct-capture confirmation.
