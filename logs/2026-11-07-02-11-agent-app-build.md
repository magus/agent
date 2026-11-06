# agent-app-build

## Context

The user requested a repeatable Agent.app build setup similar to Roomy after the signed bundle proof.

## Decisions

- Promote the minimal Rust launcher and bundle metadata; keep the Swift service separate inside
  the app. Do not add GPUI, the model loop, or permission onboarding in this change.
- Use one macOS packaging entry point and `dist/Agent.app`, following Roomy's build conventions.
- Preserve the proven bundle identifier `dev.noah.agent.proof` so the signing identity does not
  change again. Use the existing Developer ID certificate; never silently fall back to ad-hoc signing.
- Build for the host architecture with macOS 15 as the deployment target.

## Changes

- Added dependency-free Cargo configuration and `src/agent/main.rs`, promoted from the proof.
  The launcher validates arguments, starts the bundled helper, and cleans up its child on termination.
- Added `packaging/macos/app.sh` and `Info.plist`. The script selects the existing Developer ID
  identity, builds and signs both executables, and verifies the bundle before replacing output.
- Added optional `--notarize` using the existing `roomy-notary` profile, with an environment override.
- Added the helper's read-only `--permissions` diagnostic and an explicit Swift deployment target.
- Documented build, launch, stop, runtime files, signing identity, and remaining onboarding work.
  Generated app output, Cargo output, and compiler caches are ignored by Git.

## Verification

- Default build passed with Developer ID signing and strict nested signature verification.
- The first deployment-target check found Swift targeting macOS 26 despite the environment setting.
  Added an explicit Swift target; both final executables now report minimum macOS 15.0.
- Rust format and Clippy with warnings denied passed. Shell syntax, ShellCheck with the macOS
  bash-compatible `/bin/sh` dialect, plist lint, and `git diff --check` passed.
- Normal LaunchServices app launch completed the permission diagnostic without prompting.
  Both permissions are false on this laptop, as expected for the new identity here.
- Packaged launcher smoke checks passed for help, invalid arguments, helper startup, mode 0700
  runtime directory, and SIGTERM cleanup of the Swift child.
- Notarization was not run. Automatic approval review rejected the upload to Apple using saved
  credentials because the user had not explicitly authorized that external submission.
- The mini's installed proof remains unchanged. This new bundle has not repeated its locked-input
  tests on the mini; the earlier proof established that architecture and identity.

## Follow-ups

- Obtain approval to submit the built app to Apple's notarization service, then test `--notarize`.
- Implement the documented permission onboarding and capture verification separately.
