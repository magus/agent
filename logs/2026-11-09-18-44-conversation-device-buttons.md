# Conversation device buttons

## Context

The device dropdown hides access behind a menu. Show each device by its display name and make its
conversation access directly visible and clickable.

## Decisions

Show enabled devices at full contrast and disabled devices in gray. Clicking toggles conversation
access. Disable these buttons while saving to prevent overlapping permission updates.

Also remove the redundant loop-host label. Use one Send/Stop icon in the composer, with Enter
to send and Shift+Enter for a newline.

## Changes

- Replaced the device dropdown with named buttons. Enabled devices have selected styling and full
  contrast; disabled devices are gray. Tooltips describe the click action.
- Permission saves update the displayed state and block overlapping clicks.
- Removed the loop-host label and put one accessible Send/Stop icon inside the composer.
- Enter submits; Shift+Enter adds a newline. Empty text cannot submit. A draft remains unsent while
  the current turn runs; clearing it exposes Stop. Sending no longer shows Loading models.

## Verification

- Formatting, Clippy, and diff checks passed.
- Inspected the installed UI with the MacBook disabled and mini enabled.
- Shift+Enter inserted a newline without submitting. Enter submitted once and received OK.
- The icon changed to Stop during a shell sleep test. Clicking it stopped the turn. Retrieved the
  cancelled request result and left the conversation idle with mini-only access unchanged.
- Apple accepted notarization `540e0027-e0ad-4b9f-8e21-aa7e69065510`. Installed on both Macs.
