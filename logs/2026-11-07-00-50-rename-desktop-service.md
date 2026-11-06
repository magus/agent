# rename-desktop-service

## Context

The user requested `desktop_service` for the Swift component, reserving `agent` for the future
Rust agent loop.

## Decisions

- Rename the source directory, executable, entry type and file, and management script together.
- Use `~/.local/share/desktop_service` for new installations. Leave the mini's existing
  installation and historical logs unchanged.
- Keep the existing `agent.sock` protocol path for compatibility with callers.

## Changes

- Updated current source, build and launch scripts, and documentation to use the service name.

## Verification

- Compilation and installation into a dated scratch directory passed.
- Shell syntax, CLI help, and the renamed executable's usage error passed.
- The installed launcher resolved the renamed binary and respected the stop marker.
- Old source and management-script paths are absent. `git diff --check` passed.
- Desktop input tests were not rerun for this naming-only change.
