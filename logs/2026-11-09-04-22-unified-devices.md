# Unified device list

## Context

The user wants one compact device list, system-name defaults, and adding devices by address.
This is phase one of the active goal. Phase two adds the chat interface and model tool loop.

## Plan

- Unify known, discovered, and pending devices by authenticated identity.
- Separate connection status from incoming and outgoing permissions on each row.
- Add verified IP and hostname lookup and system-name defaults with optional overrides.
- Verify the UI and both Macs before beginning the agent loop.

## Decisions

- Keep existing certificates and directional grants. Address entry cannot authorize control.
- Use the macOS ComputerName as the default. Migrate only our known test names automatically.
- Keep Tailscale-specific status in connection details, not the normal device list.

## Changes

- Implementation pending.

## Verification

- Read the full goal objective and current device implementation. Live checks are pending.
