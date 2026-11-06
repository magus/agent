# move-superseded-probes

## Context

The user requested moving superseded experiments into scratch before further cleanup.

## Decisions

- Move the old Python probes and Rust Codex probe together into a dated scratch directory.
- Keep the current Swift service, scripts, and historical result logs in place.

## Changes

- Moved `probes/` and `crates/` into `.scratch/2026-11-06-19-46-superseded-probes/`.
- Replaced root README links to old probes with an overview of the current service files.

## Verification

- SHA-256 checks confirmed all 16 moved files are unchanged.
- Git status no longer lists the old experiment directories. Current source, scripts, and root
  README have no references to their former local paths.
- `git diff --check` passed. No service code changed, so desktop tests were not rerun.
