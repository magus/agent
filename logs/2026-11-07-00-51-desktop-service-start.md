# desktop-service-start

## Context

The user approved a `desktop_service start` command and reserving `agent start` for the Rust loop.

## Decisions

- Consolidate the launcher into the existing shell CLI and remove `start.command`.
- `start [install_dir]` supervises an installed service in the current Terminal session.
- Do not redeploy or restart the mini during this repository cleanup.

## Changes

- Add start, restart, and signal cleanup to the service CLI; install that CLI beside `bin/`.
- Update help and documentation to use the new command.

## Verification

- Shell syntax, compilation, and installation into a path containing spaces passed.
- A harmless child fixture verified installed default-path resolution, restart after exit 7,
  exit logging, and signal cleanup of the child process.
- Repository CLI with an explicit install path respected the stop marker. A missing installation
  exited with an error instead of entering a restart loop.
- CLI help and `git diff --check` passed. Desktop behavior was not changed or retested.
