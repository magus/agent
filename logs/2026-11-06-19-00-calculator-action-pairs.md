# Calculator action pairs

## Context

Noah requested fresh action and screenshot pairs for opening Calculator and computing `9 × 221`.

## Decisions

Use the installed native service, with both lock checks required. Observe each meaningful step.
Calculator was already open, so close and reopen it to prove the requested launch.

## Changes

Evidence is collected in `.scratch/2026-11-06-19-00-calculator-proof`.

## Verification

The initial capture passed. The service remained running and the mini remained locked.

- Closed Calculator 1112 at `(529,558)`. Screenshot confirmed it disappeared.
- Selected Terminal at `(1300,650)` and pressed Command-N. New shell 1785 appeared.
- Pasted `open -a Calculator` through the service and pressed Return. Clipboard restoration passed.
  A later screenshot confirmed Calculator reopened as window 1788. Launch used visible Terminal
  keyboard input, not an SSH launch command or accessibility control.
- Clicked AC `(540,676)`, 9 `(632,723)`, × `(678,723)`, 2 `(586,815)`,
  2 `(586,815)`, and 1 `(540,815)`. Screenshot showed `9×221`.
- Clicked equals `(678,860)`. Screenshot showed `9×221` and `1989`.
- All 13 recorded responses passed both lock guards. Screenshots are original 1920x1080 desktop
  composites from the service. The final result timestamp is Unix time 1793958755.053.
- Initial SSH authentication waited for Tailscale's check, then succeeded. No access settings changed.
- Calculator remains open with the result. The new Terminal shell remains behind it.
