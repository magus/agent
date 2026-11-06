# Locked background control

## Context

Noah challenged the overly broad conclusion about locked computer use and wants an independently
maintainable Python solution. Prior tests proved PyAutoGUI works unlocked and captures the lock
screen when locked. They did not test app-targeted Accessibility actions behind that screen.

## Decisions

- Noah reports Codex worked with his laptop locked and suspects a difference in remote use on
  the mini. Separate product-managed locked use, local GUI execution, and SSH execution instead
  of attributing every failure to the lock itself.
- Do not equate a lock-screen screenshot with proof that every app-control interface is blocked.
- Prefer a small Python experiment over reintroducing the abandoned Codex dependency.
- Keep the Mac locked, do not enter a password or alter security settings, and target Calculator only.
- Distinguish successful Accessibility control from PyAutoGUI's global mouse/keyboard control.
- Noah wants one desktop image and screen coordinates to avoid app-specific logic. Test capture
  and input separately. A failed default capture does not rule out explicit window composition.

## Changes

- Started a separate investigation and corrected the scope of the earlier conclusion.
- Added a read-only Accessibility comparison using both PyObjC and native C bindings in the
  same authorized Python process. Added a Calculator-only `CGEventPostToPid` input test.
- Added native desktop capture comparisons. Enumerate windows with
  `CGWindowListCopyWindowInfo(kCGWindowListOptionAll, 0)`, select layer-zero windows marked
  onscreen with nontrivial bounds, and pass their IDs in enumeration order to
  `CGWindowListCreateImageFromArray`. Use the main display bounds for a shared coordinate space.

## Verification

- **Live capture and input both worked while locked.** Targeted `62 × 17` produced `1054` in
  a fresh Calculator window screenshot captured by `screencapture -l 1112`. A second process
  launched while already locked entered `58 × 34` and captured `1972` in 1.356 seconds.
  That process checked the lock before input, after input, and after capture. All checks passed.
  Both result images were visually inspected.
- **One desktop-sized image of app windows worked while locked.** At Unix time 1793596680,
  the graphical-session probe reported locked=true. Explicit composition of onscreen app window
  IDs produced a 1920×1080 image showing multiple apps at their desktop positions, including
  overlapping windows. Visual inspection confirmed app content. The plain
  `CGWindowListCreateImage` onscreen capture showed only wallpaper. The explicit composition
  selects normal app windows, so it does not establish full fidelity for menus, Dock, or other
  window layers. Calculator's result was obscured by test Terminal windows in this composite.
- **Targeted input worked while locked.** At Unix time 1793596151 the local GUI process reported
  locked=true before and after sending `47 × 29` directly to Calculator's PID. Copy-result did
  not change the pasteboard. After Noah unlocked without modifying Calculator, a read-only
  Accessibility inspection at 1793596231 showed the new expression `47×29` and result `1363`.
  No arithmetic input was sent between unlocking and inspection. This proves the calculation
  executed behind the lock, though the result was verified only after unlocking.
- The locked Accessibility window list returned `AXApplication` objects instead of `AXWindow`,
  in both Python and native C bindings. After unlocking, both returned proper `AXWindow` objects
  and the numeric display. This is a lock-state difference, not merely a Python binding problem.
- These tests ran locally in the mini's graphical session. SSH only dispatched the local launch
  and retrieved output. The earlier absent-GUI-session failure under SSH is a separate issue.
- Official OpenAI locked-use docs confirm a protected temporary-unlock path exists in that product.
  They do not establish whether independently targeted Accessibility actions work while locked.

## Follow-ups

- Verify screen-coordinate mouse input while locked. Per-process keyboard input is proven;
  ordinary global input is not. A generic driver could map each point to a window and its PID,
  but that routing and inactive-window mouse behavior still need testing.
- Test capture freshness with visible changes in the composed desktop, additional window layers,
  and a supported modern capture API. The tested Core Graphics image APIs are deprecated.
- Promote the working locked Calculator probe from scratch into maintained source.
