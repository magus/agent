# device-pairing-plan

## Context

Device pairing is the next standalone feature. Remote tools belong to the later agent-loop work.

## Decisions

Separate pairing and authorization from tool execution. Use Tailscale for connectivity and explicit
device approval for access. Defer coordination between concurrent desktop controllers.

## Changes

Updated `docs/plans/agent-core/README.md` with distinct pairing and agent-loop milestones.

## Verification

Reviewed the plan against the agreed scope. Documentation only; no implementation or tests changed.
