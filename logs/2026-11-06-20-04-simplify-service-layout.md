# simplify-service-layout

## Context

The user approved two shell scripts, a lowercase source root with a service subdirectory,
and snake_case for names we control.

## Decisions

- Use `src/desktop_agent/` and preserve external macOS API names.
- Keep `start.command` as a Terminal launcher and supervisor. The extension lets Finder open
  the shell script in Terminal; it is not an app bundle or a new permission identity.
- Keep this change local to the repository. The mini's running installation need not change
  for a source layout and launcher cleanup.

## Changes

- Consolidate build and install commands in `scripts/agent`.
- Consolidate launch and restart behavior in `scripts/start.command`.
- Move and rename Swift source files and update current documentation.

## Verification

- Swift compilation and zsh syntax checks passed.
- Installed into a scratch path containing spaces. Installed executable and launcher matched
  the originals. The no-argument installed launcher respected the runtime stop marker.
- A harmless child fixture verified restart after exit, exit-code logging, SIGTERM cleanup,
  paths containing spaces, and runtime directory mode 0700.
- The sandbox denied zsh's automatic background priority adjustment with a `nice(5)` warning;
  the child still ran and all lifecycle checks passed.
- Help and invalid-command checks passed. `git diff --check` passed.
- Desktop behavior was not rerun; Swift changes only rename our types. The mini remains on
  its existing installation and keeps running.
