# Rust computer use probe

## Context

Noah chose Rust for the local assistant. The first delivery checks whether a local runner can
access shipped Codex Computer Use before attempting a harmless GUI task while locked.

## Plan

- Attempt the GUI baseline only if a supported tool connection is available.

## Decisions

- Noah explicitly approved Calculator access for this test, including subsequent trials.
  The earlier no-permission rule was too broad for the authorized task. Accept the exact
  Calculator app request through the normal tool protocol; do not broaden this to other apps
  or OS security settings. No further confirmation is needed for the same Calculator approval.
- Noah authorized SSH to `joah@joahs-mac-mini` through Tailscale. The current workstation is
  `noah-macbook-pro-m5`; its results must not be described as mini results.
- Use existing authenticated SSH access. Tailscale requested an identity check on the first
  connection outside the sandbox; that pending connection subsequently authenticated. Later
  connections succeeded. Do not repeat the check unless an actual connection requires it.
- Use Rust instead of the earlier proposed Python script.
- Identify this runner as its own client. Do not imitate a trusted desktop client or copy tokens.
- Store prototype code and build outputs in a dated `.scratch/` directory until the access path
  is proven. The unlocked tool path is now proven, so durable source lives in
  `crates/computer-use-probe`; only builds and raw runtime evidence remain in scratch.

## Changes

- Promoted the working source, fixed prompt template, and dependency lockfile to
  `crates/computer-use-probe`. Added usage and evidence documentation and linked it from README.
- Added optional `proxy` transport through the installed CLI's supported command. It retains our
  own client identity and never restarts or modifies the existing daemon.
- Added structured start and finish records, elapsed time, and explicit false verification fields.
  A successful inventory or state request cannot be mistaken for a completed Calculator task.
- Tool-level errors and missing observations now fail the probe even if the transport succeeds.
  The exact authorized Calculator request receives `accept` with empty content. Unrelated
  app-approval requests receive `decline`; attestation requests are never fabricated.
- Added a normal agent-turn mode with a fixed Calculator-only task, a 120-second deadline,
  and tool-event capture. Direct invocation has no active agent turn, so it cannot alone test
  the documented trusted-turn requirement for locked use.
- Built a Rust inventory client in `.scratch/2026-11-02-12-57-rust-computer-use-probe/`.
  It starts `codex app-server --listen stdio://`, sends `initialize` and `initialized`, then
  pages through `mcpServerStatus/list`. It identifies as `local_assistant_probe` version 0.1.0.
- The probe uses `serde_json`, imposes a 60-second response deadline, refuses server requests
  for client interaction, and terminates its child process on exit. Its output contains tool
  names and timing, not account data or tool results.
- Noah requested the detailed plan during implementation. Explained the sequence: connection,
  tool availability, unlocked Calculator task, locked and idle trials, then one iMessage test.

## Verification

- After Noah relocked the mini, the independent runner confirmed the lock flag and failed with
  `cgWindowNotFound` on a fresh `70 × 31` challenge. Runner time was 11,910 ms.
- ChatGPT, the Computer Use service, and the lock-screen guardian are running on the mini.
  `CodexComputerUseAuthorizationPlugin.bundle` exists. Presence alone does not prove correct
  locked-use setup or a trusted-turn authorization path for our client.
- `codex app-server daemon version` reports the existing daemon running at version 0.166.0.
  An attempted `daemon status` diagnostic was invalid; this CLI has no such subcommand.
- The supported proxy attempt timed out after 120,156 ms before initialization returned.
  The probe terminated its proxy child and left the shared daemon running. No GUI call ran
  through this transport. The timeout is evidence about this attempt only.
- Tests, Clippy, formatting, and whitespace checks passed for the promoted source.
- Unlocked baseline passed after Noah unlocked the mini. The normal turn selected Calculator,
  clicked the buttons for `73 × 19`, and returned UI text showing expression `73×19` and input
  `1387`. Tool calls took 389 ms and 705 ms; the turn took 8,948 ms and the runner 10,163 ms.
  This is verified UI evidence, not just the agent's final claim. Raw evidence is in scratch
  `mini-unlocked-turn.jsonl`. The runner's conservative verification flags remain false because
  result inspection is currently manual.
- Before that baseline, a normal agent turn while locked failed with `cgWindowNotFound` for
  both app-name and bundle-ID selection. Turn duration was 14,525 ms; runner time 15,427 ms.
- Noah relocked the mini for the comparison. New turns now clear old input and use a fresh
  arithmetic challenge so a stale displayed answer cannot count as a successful new action.
- After Noah's approval, direct Calculator access on the locked mini reached the tool and failed
  with `Computer Use server error -10005: cgWindowNotFound` in 2,421 ms. Approval is no longer
  the blocker. This error does not establish that all supported locked-use paths fail.
- Three focused result-validation tests passed. They cover error content, absent observations,
  and a normal observation. Formatting and Clippy passed after these changes.
  These are protocol checks, not evidence of GUI or locked-use success.
- The mini runs macOS 15.7.5 on arm64 and Codex CLI 0.166.0. ChatGPT and shipped Computer Use
  plugins are installed. Its console session reported `CGSSessionScreenIsLocked=Yes`.
- A new local inventory run with only HOME, PATH, USER, LOGNAME, TMPDIR, and LANG inherited
  still discovered the shipped tools. Inventory completed in 2,259 ms.
- The local Calculator-state attempt stopped on `mcpServer/elicitation/request`. The probe
  declined the interaction rather than granting permission. No Calculator result was obtained.
- The request on both hosts is from `cua_repl`: `Allow Computer Use to use "Calculator"?`.
  This establishes an app-approval prerequisite, not a locked-use failure.
- Copied the arm64 executable to
  `/Users/joah/.scratch/2026-11-02-13-00-rust-computer-use-probe/computer-use-probe` on the mini.
  The first copy lost executable mode and could not run; copying with `scp -p` fixed that.
- The mini inventory succeeded over SSH while locked: initialization 184 ms, four servers,
  inventory complete at 1,414 ms. `cua_repl` advertised `js`, `js_reset`, and `turn_ended`.
  The child process retains only ordinary user environment variables and uses the mini's own
  Codex configuration. No MacBook session tokens were transferred.
- The mini Calculator-state attempt reached the same app-approval request and stopped.
  The probe itself returned an error, although the remote SSH invocation reported exit code 0.
  Treat structured probe output as authoritative; do not infer success from SSH status alone.
- Cargo 1.98.1 and a cached `serde_json` dependency are available.
- Installed Codex supports app-server over standard input and output.
- `cargo fmt` and `cargo build --offline` passed.
- The sandboxed probe failed because Codex could not initialize SQLite state under `~/.codex`.
  The same probe passed with approved filesystem access. No model call or GUI action ran.
- App-server initialized in 86 ms. Inventory completed in 2,238 ms and listed 10 servers.
  These timings describe one inventory run, not model or GUI latency.
- The local client discovered `cua_repl` with `js`, `js_reset`, and `turn_ended`, plus
  `node_repl` and `cua-driver`. The separate `computer-use` entry had no tools.
  Discovery establishes visibility only; successful invocation and locked use are unproven.

## Follow-ups

- Verify a harmless invocation through the discovered shipped tool, then a Calculator baseline.
  No GUI permission, app approval, model access, or locked-use authorization has been verified.
- Compare fresh locked turns with the successful unlocked baseline. Do not change OS permissions
  or locked-use settings. The mini is currently locked again at Noah's request.
- Inspect the supported proxy handshake and whether the desktop's trusted locked-use path is
  exposed to a custom client. Never treat a handshake timeout as proof that all paths are absent.
- Idle and display-sleep success remain untested. Immediate locked operation already fails on
  the separate-server path; resolve that before spending time on longer success trials.
