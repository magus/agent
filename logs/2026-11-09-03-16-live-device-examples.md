# live-device-examples

## Context

Show two fresh examples of the paired device connection on port 47321.

## Decisions

Use direct Agent tool calls for a shell command and a locked Calculator calculation. Do not use SSH.

## Changes

Retain requests, results, and screenshots in `.scratch/2026-11-09-03-16-live-device-examples/`.

## Verification

- Remote shell returned `Joahs-Mac-mini.local`, user `joah`, and `37 × 19 = 703`, with exit code 0.
- Captured the locked desktop, cleared Calculator, and clicked `7 × 83 =`. The fresh screenshot
  shows `581`. All six input responses reported `locked: true`.
- Both examples used the paired `mac-mini` target through Agent's direct client. No SSH was used.
