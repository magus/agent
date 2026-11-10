# Unexpected Accessibility prompt

## Context

The user saw an Agent Accessibility dialog during normal use and did not intend to enable
computer use. Permission prompts should occur only during an explicit setup action.

## Decisions

- Do not ask the user to grant access to dismiss or diagnose this dialog.
- Do not claim a cause without evidence of the requesting operation.

## Changes

- Moved permission-readiness work into docs/plans/desktop-permissions/README.md at the user's
  request. It covers silent checks, revocation, rejection without prompts, and explicit setup.
  Implementation is deferred. Removed the completed agent-core plan at the user's request.

- Moved the direct tool reference to `src/agent/devices/README.md` and updated its links.
  Kept the development policy in `AGENTS.md` and deferred account work in the central-service plan.
  Existing delivery logs retain the completed work history.

- Inspected startup, desktop permission checks, tool submission, and linked Rust dependencies.
- No code change yet. The existing Swift helper checks Accessibility and Screen Recording before
  acting. Its permission status mode does not request access.

## Verification

- Confirmed permission readiness was the only unchecked agent-core item. Its original checklist
  is complete. Checked the remaining documentation links and `git diff --check`.
  No code changed, so application tests were not needed.

- User recalls clicking the model and effort pickers, not requesting computer use.
- Traced both pickers into GPUI's DropdownMenu, Popover, and PopupMenu. Opening these menus
  builds local UI and changes focus. Selection saves model settings through the chat client.
  Neither path calls the desktop service or requests Accessibility permission.
- Opened and dismissed both pickers in the running app without changing settings. No new
  Accessibility dialog appeared. A one-time or delayed dialog remains possible; this check does
  not establish the cause of the original prompt.

- No local computer-tool requests were recorded. The most recent local tool record is a shell
  request at 01:01 on November 10. This does not rule out another process or direct helper access.
- No explicit Accessibility prompt API call was found in Agent source or searched Rust dependencies.
- macOS log queries returned no matching TCC records for the reported time, including info/debug.
- Device descriptions advertise computer use unconditionally. That does not establish the cause
  of the dialog, but it fails to describe actual permission readiness.

## Follow-ups

- Reproduce the unexpected dialog with focused tracing; the reported picker interaction alone
  did not reproduce it.
- Make advertised computer-use readiness reflect silent permission checks. Keep permission requests
  restricted to explicit desktop setup actions.
