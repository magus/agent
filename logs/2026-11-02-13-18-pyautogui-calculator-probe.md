# PyAutoGUI Calculator probe

## Context

Noah abandoned the Codex Computer Use approach and requested a Calculator proof of concept with
PyAutoGUI. The prior Rust runner worked unlocked and failed while locked. Its goal is paused.

## Plan

- Prepare an isolated Python environment on the mini and a small Calculator probe.
- Check desktop visibility and permissions before sending input.
- Verify a displayed arithmetic result if Calculator is accessible. Record lock limitations.

## Decisions

- Noah prefers an owned `Agent.app` permission identity over broad terminal or Python entries.
  Keep that as the packaging direction after this proof. He explicitly enabled Python and both
  terminal apps for the current experiment; do not ask him to repeat the same grants.
- Mark the active goal blocked after the same OS-permission blocker persisted across three
  consecutive goal turns. No test job remains running, and repeating preflight cannot grant
  access. Resume the latest PyAutoGUI scope when the requesting app has both OS permissions.
  The saved goal text still describes the superseded Codex approach; do not resume that approach.
- Python is limited to this requested PyAutoGUI experiment. The assistant can remain Rust.
- Install dependencies only in the experiment's virtual environment. Do not modify system Python,
  OS permissions, locked-use settings, or other security protections.
- Do not type arithmetic into a lock screen. Verify Calculator is the visible input target first.

## Changes

- Saved the current probe and dependency pins under `probes/pyautogui` and updated the repository
  README to reflect the abandoned Codex path. Raw images and trial output remain in scratch.
- Added result verification through Calculator's Copy command, preserving and restoring the
  clipboard. This verifies keyboard automation without claiming screenshot access works.
- Noah reported completing the permission steps. Continue the PyAutoGUI experiment; the earlier
  Codex-specific objective remains superseded even though the goal status was set active again.
- Prepared a temporary, one-shot user LaunchAgent for the probe's desktop execution context.
  It lives in scratch, has no restart policy, and will be unloaded after the trial.
- Prepared `request_permissions.py` to invoke Apple's normal Accessibility and Screen Recording
  prompts from Terminal on the mini. It requests access but never approves its own prompts.
- Began this separate experiment. Existing Codex proof and evidence remain in place.
- Created `calculator_probe.py` and pinned requirements in the dated local scratch folder.
  A matching folder on the mini contains a virtual environment and the test script.
- The probe checks session availability and OS permissions before opening Calculator or typing.
  A missing graphical session is unknown lock state, never treated as an unlocked session.

## Verification

- Locked observation completed at Unix time 1793595525 after Noah locked the mini. The probe
  reported locked=true, Accessibility=true, Screen Recording=true, and two enumerated Calculator
  windows. The inspected screenshot shows the macOS lock screen and password field, not the app.
  Enumeration of Calculator windows therefore does not establish an available input target.
  No input was sent. This confirms that the observed screenshot/keyboard path does not expose
  Calculator while locked; it does not test every alternative automation or virtual-desktop path.
- The first lock observer exited when Terminal restarted. Confirmed its process was absent
  before starting the replacement; did not treat an observation timeout alone as process death.
- Python syntax checks and repository whitespace checks passed for the saved probe.
- Unlocked PyAutoGUI baseline passed at Unix time 1793595360. The fresh process reported
  Accessibility=true and entered `83*27` in the frontmost Calculator app. A new clipboard change
  yielded the expected `2241`; the existing clipboard was restored. Total time: 3,260 ms.
- Inspected the actual PyAutoGUI screenshot. It shows wallpaper and the Calculator menu bar,
  but no app windows. Screenshot file creation therefore did not establish screen-recording
  access. The earlier optimistic interpretation of screenshot creation is superseded.
- After Noah's latest approval, one attempted launch left the result file unchanged. Its output
  was stale and must not be used to claim current permissions were denied. Added a timestamp and
  executable path, and used a fresh launcher to verify a new process actually ran.
- The fresh Terminal process reports Accessibility=true. Quartz screen-capture preflight still
  reports false, but PyAutoGUI's separate `screencapture` backend successfully wrote an image.
  Removed that preflight boolean as a hard gate; inspect the actual screenshot before input.
  This corrects the earlier claim that both permissions necessarily blocked the PyAutoGUI path.
- Rechecked the Terminal-launched probe after the latest goal continuation. The output file was
  freshly updated at Unix time 1793594793. The desktop is unlocked and Calculator is visible;
  Accessibility and Screen Recording still both report false. No input was sent and no new
  permission prompt was opened. Further GUI testing requires the actual OS grants to change.
- A normal user LaunchAgent successfully entered the unlocked GUI session and saw two
  Calculator windows, but both OS permission checks remained false. The test sent no input.
  Unloaded the one-shot job after it exited.
- Launched the probe via a Terminal `.command` file. It also saw the unlocked GUI session and
  Calculator windows, but both permission checks remained false. No arithmetic was entered.
- Updated the permission helper to load the same AppKit and PyAutoGUI imports as the probe,
  then opened it from Terminal to request access for that runtime. This tests a possible
  process-attribution difference; it is not a proven explanation of the denied checks.
- The saved permission-helper result reports Screen Recording=true and Accessibility=false.
  The latter may have been measured before Noah finished approving access; recheck live state.
- SSH still has no graphical session and both permission checks false after the user steps.
- `launchctl asuser 501` failed with `Operation not permitted` while switching audit sessions.
  Do not retry it with elevated privileges. Test a normal user LaunchAgent instead.
- Noah unlocked the mini. The corrected SSH preflight still returns no graphical session,
  no visible-window inventory, and both permission checks false. This is an SSH execution-context
  limitation at this stage, not evidence that the user left the screen locked.
- The mini is locked. Its default `/usr/bin/python3` has no PyAutoGUI, Quartz, or Pillow modules.
- Installed PyAutoGUI 0.9.54 and Pillow 11.3.0 into the isolated environment. The original
  resolver chose a yanked PyObjC 12.0 release with incorrect Python 3.9 metadata. That install
  finished before an attempted interrupt; the interrupt failed because its process had exited.
  Replaced all three PyObjC packages with compatible 11.1 wheels successfully.
- The SSH preflight reports Accessibility=false, Screen Recording=false, no window inventory,
  and zero observed Calculator windows. Its first version mishandled a null window list; fixed.
  Its first lock report also treated a missing session as false; corrected to unknown.
- No input was sent. These observations do not prove that PyAutoGUI can or cannot work locked
  after proper permission and session setup. They establish missing prerequisites for this run.

## Follow-ups

- The requested PyAutoGUI proof is resolved: unlocked calculation verified; locked screenshot
  exposes the lock screen. No further permission request is pending and no test job is running.
- Before building a full assistant, choose a path that does not require the locked desktop,
  such as a suitable app interface or isolated desktop. Do not claim these alternatives work
  until separately tested. A dedicated signed Agent.app remains the preferred permission owner.
