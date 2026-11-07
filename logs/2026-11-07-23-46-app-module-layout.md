# app-module-layout

## Context

Group Agent under app/ and keep every mod.rs limited to declarations and exports.

## Decisions

- Keep Args under app because the arguments select app startup behavior. App parses them;
  main only runs the app and reports an error.
- Keep desktop service and ChatGPT under app, with shared window and storage code under core.
- Preserve descriptive implementation filenames and ordinary Rust module resolution.

## Changes

- main.rs now declares only app and calls App::run(). App parses its arguments internally.
- Moved startup code to app/application.rs, Args to app/args.rs, and both features beneath app/.
- Moved shared storage and window code to app/core/. Extracted ChatGPT registration into
  register.rs. Every mod.rs contains only declarations and imports/exports.
- Kept desktop implementation in desktop_service/service.rs to avoid repeating the module name.

## Verification

- All six tests passed after the move. Final formatting, Clippy across all targets, and diff
  checks passed. Clippy initially caught redundant visibility and a repeated module name; both
  were corrected. No path overrides were added.
- This changes source organization only. The running app was not rebuilt or restarted.
