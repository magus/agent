# remove-standalone-launcher

## Context

Agent.app now owns the desktop service. The user approved removing its older standalone launcher.

## Decisions

- Compile Swift directly in the app packaging script. Keep one documented app launch path.
- Keep `src/agent/main.rs` as the Rust app entry point. It manages the Swift child process;
  it does not implement a model loop.

## Changes

- Removed `scripts/desktop_service` and moved its compilation command into `packaging/macos/app.sh`.
- Updated build and request documentation to use Agent.app and its runtime directory.
- Left the installed mini app and its permission grants unchanged.

## Verification

- Shell syntax check passed. Executed the exact Swift compilation command from the packaging
  script with its macOS 15 target; compilation passed.
- No remaining references to the deleted script in current source, packaging, or README files.
  Historical logs retain their original commands. `git diff --check` passed.
- Did not rebuild, notarize, or reinstall Agent.app; signing and runtime code did not change.
