# Unified device list

## Context

The user wants one compact device list, system-name defaults, and adding devices by address.
This is phase one of the active goal. Phase two adds the chat interface and model tool loop.

## Decisions

- Keep existing certificates and directional grants. Address entry cannot authorize control.
- Use the macOS ComputerName as the default. Preserve existing names as overrides during migration.
  Reset our two assigned test names through the local administration API during verification.
- Keep Tailscale-specific status in connection details, not the normal device list.

## Changes

- Added a compact unified table with separate incoming and outgoing controls.
- Added verified IP and hostname lookup using the existing discovery authentication.
- Added a Tailscale-name fallback when system DNS cannot resolve a hostname. Ambiguous names fail
  instead of selecting a device. The same identity checks still apply.
- Added a system-name default, optional override, and Use system name control.

## Verification

- All 24 tests passed, including identity deduplication and keeping offline devices visible.
  Clippy passed with warnings denied.
- Inspected the local unified table. Connected stays inline, and both permission directions have
  separate controls. The system-name button restored `noah-macbook-pro-m5` from macOS ComputerName.
- Hostname lookup verified the existing mini identity without adding a duplicate row or granting
  access. Verification feedback is visible directly below the address field.
- IP and hostname lookup passed on both Macs after adding the verified Tailscale-name fallback.
- Inspected connected, pending, declined, offline, and empty states. Known offline devices remain
  visible. Inspected the compact table on the locked mini through its desktop service.
- Both apps were signed, notarized, installed, and restarted. System names and device identities
  survived. Name overrides and returning to the system name passed on both Macs.
- MacBook-to-mini tools still work. Mini-to-MacBook tools are rejected. Temporary reverse requests
  and empty-state test data were removed, and the original grants were restored.
