# `AGENTS.md`

## Must read

Read `.agents/AGENTS.md` before starting work. Apply its instructions to this project's root.
If `.agents/AGENTS.md` is missing, run `git submodule update --init --recursive` first.

## Development policy

Backward compatibility is not required during early development. Remove unused fields and obsolete
paths rather than preserving them for older builds. Preserve user data unless a change requires an
explicit reset. Compatibility becomes a requirement only when explicitly agreed.
