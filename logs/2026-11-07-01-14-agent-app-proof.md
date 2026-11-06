# agent-app-proof

## Context

The user approved a minimal Agent.app with a Rust launcher and existing Swift desktop helper.
The proof must establish app-owned permissions, retention after an update, and locked control.

## Decisions

- Keep experimental bundle code in dated scratch until the permission model is verified.
- Preserve the installed mini service while testing a separate app and runtime directory.
- The initial restricted keychain check incorrectly reported no local identities. The user
  pointed to `/Users/noah/code/roomy`; its signing notes explain the sandbox limitation.
  An unrestricted check found the existing Developer ID Application identity for team 78Z6KJV687.
- Reuse that identity without exporting private keys. Compile Swift on the mini, copy the helper
  here for signing, then transfer the signed app back.

## Changes

- Created a Rust launcher, app metadata, and a copy of the Swift helper with read-only permission
  diagnostics in `.scratch/2026-11-07-01-14-agent-app/`.
- Installed the Developer ID signed proof at `/Users/joah/Applications/Agent.app`.
  Bundle ID is `dev.noah.agent.proof`; runtime is
  `/Users/joah/Library/Application Support/AgentProof`. The original service remains untouched.
- Prepared and installed a second signed build with a changed Rust executable and bundle version 2,
  after the initial permission grants and successful build 1 input test.

## Verification

- Local Rust compiler is available. SSH to the mini succeeded without another authentication request.
- Rust compilation with warnings denied, native Swift compilation, plist lint, and strict nested
  code-signature verification passed.
- Initial normal app launch failed because the copied Rust executable lacked execute permission.
  Corrected its mode to 0755; subsequent LaunchServices launches passed.
- macOS TCC logs identified `dev.noah.agent.proof` as the responsible app and permission subject
  for requests originating from the Swift helper. This supports the single-bundle design.
- Both permission preflights returned false. Before grants, capture returned success but its image
  contained only wallpaper and menu bar, not app windows. That is not a successful desktop proof.
- Signed builds 1 and 2 have different executable SHA-256 hashes but identical designated
  requirements: bundle identifier plus Apple Developer ID constraints and team 78Z6KJV687.
  This verifies signing identity continuity, not actual permission retention.
- After the user's initial grants, Screen Recording became true and locked capture showed app
  windows correctly. Accessibility stayed false; a Calculator click failed at window focus with
  error 1002, so no successful input was claimed.
- TCC logs showed Accessibility's saved requirement was the earlier ad-hoc cdhash, while the
  installed app had the Developer ID requirement. This was stale test identity registration,
  not evidence that a consistently signed update loses permissions.
- Unregistered superseded remote test copies, renamed the ad-hoc bundle to `Agent.app.disabled`,
  and registered only `/Users/joah/Applications/Agent.app`. Asked the user to remove and re-add
  Agent in Accessibility at that exact path. Screen Recording needs no change.
- After the user corrected Accessibility, both permission checks returned true. The next input
  request was rejected because the session was unlocked; asked the user to lock it again.
- Investigated the additional direct-capture alert. Apple documents the managed restriction
  `forceBypassScreenCaptureAlert` for macOS 15.1+, with manual profile installation disallowed
  in its device-management schema. The persistent-content-capture entitlement requires Apple's
  approval and is described for VNC apps; eligibility is not established for this project.
  Changing to ScreenCaptureKit alone does not establish unattended prompt-free operation;
  Apple's migration advice includes the interactive SCContentSharingPicker.
  Sources: https://github.com/apple/device-management/blob/release/mdm/profiles/com.apple.applicationaccess.yaml
  and https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.developer.persistent-content-capture
  and https://developer.apple.com/documentation/macos-release-notes/macos-15-release-notes
- After the user relocked the mini and closed RustDesk, build 1 captured the full desktop,
  cleared Calculator by coordinate click, and entered `47 × 29 = 1363` using key events.
  Screenshot inspection confirmed the result. Both lock flags were checked by each action.
- Stopping the Rust parent cleaned up its Swift child. Restarting build 1 preserved the grants.
- Stopped the proof app, installed build 2 at the same Applications path, and verified its signature.
  Both Accessibility and Screen Recording remained true without another permission change.
  Build 2 cleared Calculator and entered `62 × 17 = 1054`; screenshot inspection confirmed it.
  All successful capture/input responses reported locked=true. The original service was untouched.
- Build 1 Rust executable SHA-256: `18f24cfb2a08a5a37e18a61e748255b6e74b2858ee3de46d669b2fb3b515da5f`.
  Build 2: `8b8ee1b660ecbcdc18311ad82e487311eae67bcd69c923b1c5be93828bab39f4`.
  The helper binary did not change in this update test.
- Evidence files under the proof scratch directory include `build1-before.jpg`, `build1-clear.jpg`,
  `build1-result.jpg`, `build2-clear.jpg`, `build2-result.jpg`, matching JSON responses, and
  `build2-verification.txt`. Bundle v2 remains running on the mini.

## Follow-ups

- The single-app permission attribution and changed-build retention proof passed after correcting
  the stale ad-hoc Accessibility entry. No permission databases or grants were modified programmatically.
- Recurring capture alerts remain unresolved. The mini is not enrolled in MDM. No claim is made
  of indefinite prompt-free operation, reboot persistence, or a complete assistant runtime.
- Promote the proven packaging approach out of scratch in a separate implementation step.
