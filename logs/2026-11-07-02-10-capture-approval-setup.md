# capture-approval-setup

## Context

The user chose an interactive app setup flow to trigger the extra macOS capture confirmation,
have the user approve it, and verify capture before unattended work. They requested documentation.

## Decisions

- Use this setup flow for the first version instead of requiring MDM or waiting for Apple's
  Persistent Content Capture entitlement approval.
- Verify actual captured content. Neither the Screen Recording toggle nor a successful API
  response alone establishes usable capture or acceptance of the extra confirmation.
- Treat later permission interruptions as recoverable. Do not promise permanent suppression.

## Changes

- Added the setup sequence, runtime behavior, acceptance checks, and limits to the root README.
- Marked the flow as planned, not implemented, and linked it from the service documentation.

## Verification

- Checked the text against the user's decision and the proof results: capture previously returned
  success with wallpaper only, and grants survived the tested consistently signed app update.
- Documentation-only change; no code or running app changed.
