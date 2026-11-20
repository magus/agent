# Pierre Words markdown plan

## Context

The user wants Pierre Words tracked for Agent's markdown rendering and prefers Pierre's products.
This request is to record the direction and implementation steps.

## Decisions

- Use Words as the target markdown renderer. Prove its integration with the native Mac and iPhone
  clients before replacing their existing Swift rendering.
- Keep the native app architecture. A web-based content renderer does not imply a web-based app.

## Changes

- Add a dedicated markdown rendering plan, linked from TODO and the active native UI plan.

## Verification

- Reviewed Pierre's official Words documentation at `https://diffs.com/docs` after the demo URL
  could not be opened by the web tool.
- Located current Swift markdown rendering and the active native UI migration plan.

## Follow-ups

- Implement the integration proof and remaining checks in the markdown rendering plan.
