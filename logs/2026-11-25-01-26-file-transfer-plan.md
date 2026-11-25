# File transfer plan

## Context

Moving files between devices relies on Tailscale SSH plus `scp`, or serving files on the tailnet
temporarily. Noah wanted a first-class tool. The work was tracked in a one-line TODO (upload only)
and as an unchecked item in the remote maintenance plan. This change writes the plan only;
implementation has not started.

## Decisions

- One symmetric `copy` operation over the existing paired TLS device connection, naming a device
  and path for both source and destination. It covers upload, download, and device to device; for
  device to device the caller relays chunks, so directional grants stay unchanged.
- Send bounded chunks with a SHA-256 check instead of raising the 64 MiB message limit.
- Preserve file mode (the executable bit `scp` dropped) and support directories.
- Permissions: the caller needs shell-level access on both ends, since shell already gives full
  filesystem access. This replaced a first draft that only checked the remote device, which did not
  cover writes to the caller's side. Devices without a shell use a dedicated transfers folder.
- Keep a free-space reserve on the destination, 5 GB by default, checked before and during writing.
- Show every transfer in the conversation and history like a shell call (paths, size, hash, outcome).
- Provide a development CLI command and a model tool built on one operation.

## Changes

- Added `docs/plans/file-transfer/README.md`.
- Replaced the upload TODO in `TODO.md` with a pointer to the plan.
- Linked the plan from the file transfer item in `docs/plans/remote-maintenance/README.md`.

## Verification

- Documentation only; no code changed or tested.

## Follow-ups

- Implement the plan, starting with the protocol operation.
