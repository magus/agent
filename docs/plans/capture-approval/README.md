# Capture approval setup

The Rust app now provides a GPUI setup window. It shows permission status, opens Settings, and
asks the user to confirm a real capture preview. This first version uses interactive setup instead
of device management or Apple's Persistent Content Capture entitlement.

The goal is to handle the initial capture confirmation while the user is present, before unattended
work begins. The ordinary Screen Recording grant and the extra confirmation for direct capture are
separate checks. Accessibility is also required for desktop input.

### Setup flow

1. Launch the installed, consistently signed Agent.app and its bundled desktop service.
2. Guide the user through granting Accessibility and Screen Recording to Agent.
3. Explain that macOS may show an additional direct-capture confirmation. Start a real capture
   through the same helper and capture method used during normal operation to trigger it.
4. Have the user click Allow in the macOS dialog if it appears. Agent must not click it for them.
   If no dialog appears, continue with verification; do not assume setup failed.
5. Verify a fresh image includes expected app content, not just wallpaper or a menu bar. Show a
   preview for the user to confirm. A successful API response alone is insufficient.
6. Mark capture setup ready only after verification succeeds. If approval is declined or capture
   remains unusable, keep setup incomplete and offer another attempt when the user is ready.

Normal service requests reject unlocked capture and input. A separate setup command provides
capture only when the Mac is unlocked, without removing the locked-execution checks.
After setup, verify capture and input again with the Mac locked.

### During unattended work

- Check capture readiness at startup and before resuming desktop work. Save the last successful
  verification for diagnostics, but do not treat that saved state as permanent permission.
- If usable capture is lost, pause desktop actions and request user attention. Do not continue
  clicking from an old screenshot or automatically repeat an action that may already have run.
- Report approval as the cause only when diagnostics establish it. Otherwise report capture as
  unavailable, preserve the error, and offer the setup check again.
- After the user resolves the issue, obtain a fresh verified capture before resuming.

### Limits and acceptance checks

We have not established a public API that separately reports acceptance of the extra confirmation.
The Screen Recording preflight is not proof that the user clicked Allow. Successful capture proves
current functionality, not that the dialog cannot appear again. macOS may request confirmation later;
this setup flow does not guarantee permanent suppression.

Verify initial approval, decline and retry, already-approved startup, app restart, a consistently
signed update, and locked operation. Also verify that loss of usable capture pauses desktop actions
without replaying input. Keep failure detection conservative. A sparse desktop image alone does not
prove a permission failure.

### Verified

- The user enabled both local grants and accepted the additional capture dialog.
- Preview, rejection, retry, confirmation, restart, and signed updates retained working capture.
- Closing setup kept the service running. Reopening Agent restored the setup window.
- The mini retained both grants after the GPUI update. Locked capture, clicks, keys, and window
  movement passed. A stale target failed before input, and further input required a new capture.
- Async setup changes redraw without another click, including on the locked mini.

### Remaining checks

- Declining the macOS capture dialog and revoking a grant have not been tested. Rejecting the app's
  preview was tested, but it is a separate action.
- A future agent loop must request user attention when capture fails. The service currently returns
  the failure, clears capture readiness, and refuses further input until a new capture succeeds.
- Recurring macOS confirmation remains possible. The saved confirmation does not suppress it.
