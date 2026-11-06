# readme-cleanup

## Context

The root README mixed usage, design plans, implementation details, and test history.
The user approved keeping it short and moving necessary reference material beside its subject.

## Decisions

- Keep the root README to a description, build commands, and reference links.
- After review, the user requested Roomy's root README style. Follow its lowercase headings,
  short description, service-reference quote, setup, development, packaging, and version sections.
  Omit Roomy's scanner tests because Agent does not use them.
- The user requires Roomy's full Xcode and Metal toolchain setup. Copied those instructions
  exactly, replacing the Command Line Tools-only setup. This supersedes the earlier decision
  to omit Metal based on the current service's dependencies.
- Removed the minimum-macOS sentence from the root README at the user's request.
  The deployment target remains documented in packaging and configured in the build and plist.
- Mirrored Roomy's development wording, Rust check commands, and cargo-edit version commands
  after the user pointed out those differences. Removed extra development prose.
  Omitted plain `cargo run`: it does not build or place the required adjacent Swift helper.
- At the user's request, replaced the vague permission/service link with first-launch permission
  steps directly in the root README, including the possible capture confirmation and lock requirement.
- The user dislikes colons in prose. Removed the colon after the first-launch introduction.
- Moved permission setup into its own section because it applies to all builds. Refer to the
  app copy being used rather than assuming the development output path.
- Keep test evidence and old identity or installation details in existing delivery logs.

## Changes

- Added `packaging/macos/README.md` for build, signing, and notarization instructions.
- Moved the agreed, unimplemented capture flow into `docs/capture-approval.md`.
- Kept service commands, diagnostics, and current limits in `src/desktop_service/README.md`.
- Removed repeated test history and the file-by-file project inventory from reference docs.

## Verification

- All local links in the four reference documents resolve. `git diff --check` passed.
- The root README now follows Roomy's structure with Agent's actual prerequisites and commands.
  No executable code changed.
- Documented format, Clippy, and Cargo test commands passed. Cargo currently reports zero tests;
  these checks do not replace the recorded locked-desktop proof.
