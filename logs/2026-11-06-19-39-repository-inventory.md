# repository-inventory

## Context

The user requested an inventory before cleanup because many new folders are untracked.

## Decisions

- Review the repository and mini installation without moving or deleting files.
- Treat the Swift service as current code and the Python and Rust probes as historical experiments.

## Changes

- Added this inventory log. No implementation, installation, or running process changed.
- Before this log, Git showed 34 untracked files and two modified files. The tracked baseline
  contains project instructions, the instruction submodule, ignore rules, and a minimal README.
- Current implementation has three Swift files totaling 713 lines and four shell scripts totaling
  64 lines. It has no package dependencies. Python and Rust probes are separate experiments.
- Local ignored scratch data uses about 140 MB. Build output uses about 64 MB, mostly compiler cache.
- The mini installation uses about 75 MB. Dated remote experiment folders also remain.

## Verification

- Read Git status, tracked paths, source, scripts, and probe documentation.
- Inspected local and remote directory sizes. The remote process-name check found the installed
  desktop-agent and its shell supervisor, with no matching old probe processes.
- This was a structural review. No new desktop behavior test or full correctness audit ran.

## Follow-ups

- Separate retained historical probes from the current service so the project has one clear entry.
- Add screenshot retention and log rotation. Capture currently creates a new UUID image per request.
- Reduce mixed responsibilities in DesktopControl.swift without adding a framework.
- Consolidate repeatable proof checks; current evidence is spread across logs and scratch scripts.
- Package a named app with its own permission identity and startup behavior. The current launcher
  still relies on Terminal permissions.
- Preserve known limits in current documentation, including cross-app menu activation,
  undocumented macOS APIs, and the proof's requirement that the desktop remain locked.
