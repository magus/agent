# permission-setup

## Context

The user approved implementing the small Agent permission setup flow and testing it on this Mac.
The final signed app must retain locked desktop control on the mini.

## Decisions

- The first AppKit window compiled and opened, but the user requested GPUI for the interface.
  Replace that window with Rust and GPUI Kit, matching Roomy's UI dependency. Keep Swift for
  permission probes, opening settings, and capture. Preserve the discarded UI only in scratch.
- Keep unlocked capture exclusive to a user-triggered setup action. Do not expose unlocked input.
- Require a fresh preview and user confirmation. A saved confirmation is not permanent permission.
- Preserve the app and helper signing identities.
- The user found duplicate UI from Open Settings. Remove explicit permission-prompt calls from
  those buttons; open only the relevant settings page. Users add Agent.app if it is absent.

## Changes

- Added `PermissionSetup.rs` for GPUI permission status, settings actions, preview, and confirmation.
- Added `DesktopService.rs` for child lifecycle and native helper requests. The Rust app owns menus,
  window reopening, and background operation.
- Added explicit native setup-capture and settings-request modes. Socket input retains lock guards.
- Added permission checks and a fresh capture before input. Failed actions clear capture readiness
  and selection; requests are never replayed automatically.

## Verification

- GPUI release build and Developer ID signing passed. Clippy passed after correcting a unit binding.
- Inspected the GPUI window screenshot. Both local grants correctly showed missing and capture was
  disabled. The Accessibility button opened the macOS prompt naming Agent.app.
- Local socket capture, empty key input, and setup-capture requests rejected the unlocked session
  with `automatic_retry` false.
- The older CUA app resolver still used the old proof identity. A direct window inspection by PID
  found and captured the new GPUI window.
- Automatic approval review rejected clicking the macOS Open System Settings prompt as a grant.
  Asked the user to enable the initial local grants; did not change permissions.
- The user enabled both local grants, restarted Agent, and clicked the extra capture Allow dialog.
  Verified real app content in the preview, rejection, retry, confirmation, and saved confirmation
  after restart. Signed rebuilds retained both grants.
- Async permission probes completed but GPUI did not redraw until input. App activation changes and
  GPUI refresh calls alone did not fix it. A native layer redraw request on view changes fixed status,
  preview, and confirmation updates. Unchanged permission polls do not request redraws.
- Closing setup kept both processes running. Reopening Agent restored setup. Quitting stopped the
  helper. Final formatting, Clippy, Swift compilation, and signing passed. Cargo test passed with
  zero tests; the behavior checks above were manual integration tests.
- The mini retained both grants after the update. Locked capture, Calculator clicks, `83 × 27 = 2241`,
  and moving Calculator passed. A stale target failed before input; subsequent input was refused
  until a fresh capture succeeded. The final build calculated `9 × 221 = 1989`; every action returned
  `locked=true`, and both setup statuses rendered Enabled.
- The earlier GPUI build passed notarization and stapling, submission
  `7253d94b-7d1b-4ee1-9874-1ae28d65e276`. Final notarization stopped before submission because
  `notarytool` could no longer find `apple-notary`, including with the login Keychain specified.
  Asked whether the profile moved. The final build is Developer ID signed but not notarized.
- Installed the final signed app at `/Users/joah/Applications/Agent.app`. Preserved the earlier
  notarized build in the mini's task scratch directory as `Agent-before-redraw.app.disabled`.
  Initial plain `scp -r` lost executable bits; the final transfer used `scp -pr` and checked both
  executables before installation.

## Follow-ups

- Resolved in [setup cleanup](2026-11-07-13-31-setup-cleanup.md). `apple-notary` authenticated
  successfully on recheck; no credential change was needed.
- OS-level denial and permission revocation remain untested. Preview rejection is not an OS denial.
- Future unattended-loop work must turn capture failures into user attention. Recurring confirmation
  is still possible and is documented in the capture-approval plan.
