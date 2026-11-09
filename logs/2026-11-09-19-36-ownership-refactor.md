# Ownership refactor

## Context

Separate the desktop interface, central execution, device communication, shared protocol, and OS
facilities without changing runtime behavior. Preserve all existing uncommitted changes.

## Decisions

- Keep one binary. Dispatch commands before GPUI startup.
- Keep UI and browser callbacks in app; keep inference, credentials, persistence, and recovery in service.
- Put shared conversation data and service message types in protocol. Shared modules cannot import
  app, service, or GPUI.
- Both desktop and central startup use DeviceService and ToolConnection, so devices stays shared.
- The initial refactor preserved serialized fields. The user has since clarified that backward
  compatibility is not required during early development. Remove unused fields while preserving
  actual conversation history, identities, and grants.

## Changes

- Moved device communication and endpoints to devices, central execution to service, and the
  runtime directory lookup to platform.
- Split service Conversation persistence from protocol ConversationView, Entry, and ToolRequest.
  Shared catalog, response, and request types retain their serialized field names.
- Moved ChatGPT credential handling into service/chatgpt. Browser callback handling stays in app.
- Added root command dispatch in run.rs; App::run now only starts the desktop interface.
- Updated source ownership documentation. No crate, executable, transport, or migration was added.

- Split DeviceService into service state, discovery, pairing, connection handling, request messages,
  and tests under devices/device_service/. mod.rs contains only declarations and exports. Keep the
  app/app.rs naming convention for device_service/device_service.rs.
- ToolConnection reads local identity and allowed devices through typed accessors, not JSON snapshots.
- Removed loop_host from persisted conversations and UI views. Model instructions read the service
  identity directly. Removed the empty input_list from UI views; inference history remains on the
  service. Renamed protocol compatibility tests to describe current request and response behavior.
- Recorded the early-development compatibility policy in docs/plans/agent-core/README.md.

## Verification

- All 35 tests passed, including the existing 32 and three protocol compatibility checks.
  One new test initially assumed Serde rejects unknown fields on unit variants; corrected the test
  to check a structured request without changing the existing parser behavior.
- Clippy passed. Dependency scans found no app/service cross-imports or GPUI in shared modules.
- Credential/inference code, tool schemas, and model instructions are unchanged byte-for-byte.
- Formatting and diff checks passed. Swift control code, packaging scripts, and bundle template
  are unchanged. Signature verification retained `com.iamnoah.agent` and team `78Z6KJV687`.
- Apple accepted notarization `fad0dbad-d346-4078-a02f-0ddf467aec20`; installed on both Macs.
- The mini's service answered catalog and account requests while its desktop app was closed.
  Desktop startup and the existing CLI commands also worked after installation.
- Both clients matched the saved pre-refactor catalog, device identities, pairing grants, creation
  defaults, conversation entries, tool request IDs, images, and central signed-in state.
- Inspected the desktop chat and confirmed Accessibility and Screen Recording remain enabled on
  the MacBook. No permission grants or login were repeated.
- Central inference used the existing account and dispatched one authorized mini shell request.
  It returned `Joahs-Mac-mini.local`. Earlier history and grants were left intact.
- Cleanup verification passed all 39 tests, formatting, Clippy, and diff checks. The directional
  revocation test now checks typed local identity and target lists as well as remote authorization.
- Apple accepted cleanup build `3daf6083-6938-414a-b6f3-961d2112055e`. Installed on both Macs.
  Desktop chat reopened. Both clients retrieved the shared catalog. Saved conversation entries and
  grants matched the pre-update snapshot. Central account access remained available, and an authorized
  mini shell request returned `Joahs-Mac-mini.local`. UI responses no longer contain either removed
  field. No inference request was needed for this cleanup check.
