# Prototype readiness

## Context

Noah asked to read the repository instructions and assess readiness to build a minimal local
runner that uses shipped Codex Computer Use while the Mac mini is locked.

## Decisions

- Start with one harmless Calculator task through a local Codex runner. Prove tool access
  unlocked before testing locked use. Defer iMessage handling until this works.
- Treat access to shipped Computer Use and trusted locked-use authorization as unverified.
  Official documentation limits locked use to trusted Computer Use turns. A custom client
  must not assume it inherits that access or imitate a trusted client.
- Keep experiments in dated `.scratch/` folders and durable findings in tracked logs.
  Do not install software, grant permissions, change security settings, or send test messages
  as part of this readiness check.

## Changes

- Recorded the current scope and readiness here. No prototype has been implemented.
- The earlier plan and README content are absent from the current checkout. Preserve the
  current repository setup instead of restoring those files without a reason.

## Verification

- Read root `AGENTS.md` and `.agents/AGENTS.md`; the latter applies at the project root.
- Confirmed Python 3, Codex CLI 0.144.1, and macOS 26.6.2 are available.
- The working tree was clean before this log was added.
- No GUI or locked-use trial has run. Authentication and permissions remain unverified.

## Follow-ups

- Inspect supported local access to the shipped Computer Use tools and build the smallest
  runner if available. Record the missing capability if that path is unavailable.
- Confirm the execution host is the target mini before a physical lock trial.
- Run an unlocked baseline, then delayed locked, idle, display-sleep, and runner-restart trials.
  Require GUI tool evidence and observed output, not just the model's reported success.
- Inspect the old bridge on the mini before proposing code reuse. The attachment's
  `/Users/joah/code/dots` path was unavailable during the earlier planning check.
