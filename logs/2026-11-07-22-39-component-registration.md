# component-registration

## Context

Isolate component setup instead of moving wiring into another large entry point.

## Decisions

- Use App and Args. Keep DesktopService and the existing ChatGPT names.
- Each component registers its actions, menu item, and startup behavior.
- Share the runtime directory without making ChatGPT depend on DesktopService.

## Changes

- Replaced AgentApp.rs with App.rs and moved command parsing into Args.rs. App registers the two
  components and handles application-wide quit, reopen, and startup errors.
- DesktopService owns helper creation, supervision, shutdown, permission diagnostics, and its
  setup action. The chatgpt module owns client creation and its account-window action.
- Components return their menu entries to App. Their constructors and window dependencies stay
  private to their registration code. No component trait or plugin framework was introduced.
- Extracted runtime_dir.rs so both components use the same storage location independently.

## Verification

- Formatting, Clippy, diff checks, and all six tests passed. The new argument test covers the
  existing commands, default setup, and rejection of unknown or conflicting arguments.
- Notarization and ticket validation passed, submission `155e7cba-7c56-4b61-ab76-dec11647236c`.
- Launched the signed app with --chatgpt and confirmed the saved account loaded. Exactly one
  helper ran under that process. After SIGTERM, neither PID remained; ps returned 1 with no rows.
- Relaunched without arguments and verified the permission setup window. The updated app is
  running locally. This refactor was not deployed to the mini.
