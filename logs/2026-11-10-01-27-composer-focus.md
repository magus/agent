# Composer focus

## Context

Typing after switching to Agent or tabbing to a button can do nothing when the composer is not
focused. Conversation text also needs selection and copying.

## Decisions

- Focus the composer on printable typing from non-text controls, including tab-focused buttons.
  The initial no-focus-only condition was too narrow.
- Make message bodies and role labels selectable, including plain user and tool text.
- Shift+Esc focuses the composer directly. Preserve existing drafts and the first typed character.
- Keep command shortcuts, editable inputs, and open menus handling their own keys.

## Changes

- Added a keyboard handler scoped to the chat window. Its subscription ends with the window.
- Printable input from unfocused areas or tab-focused buttons focuses the composer and inserts the
  first character once. Input and PopupMenu contexts keep their normal behavior.
- User and tool text now use SelectableText. Assistant Markdown explicitly enables selection.
  Starting a selection in history releases composer focus. The chat keyboard handler copies the
  selected history text when no input or menu owns focus. GPUI’s normal Copy action did not run
  in that unfocused state during the live test.
- Shift+Esc focuses the composer. Updated keyboard instructions in the root README.

## Verification

- Formatting, Clippy, signing, and notarization passed.
- Live test after launch typed `abc` without clicking the composer; all characters appeared once.
- Opened the model menu, pressed Shift+Esc, then typed `d`; the existing draft became `abcd`.
- Command+A and Backspace cleared only the test draft. No messages were sent.
- Live test tabbed away from the composer, then typed `qw`; both characters appeared once.
- Selected and copied user text and assistant Markdown, then pasted into the composer. Both
  matched the selected text. Copying within the composer still worked. Cleared all test drafts.
- Confirmed conversations were idle before updating the mini. Installed the signed, notarized
  build on both Macs. The mini app and central service restarted successfully. Submission
  `7b307f1f-8879-48e0-a5bb-d3079d6210f3` was accepted.
- Final source cleanup only added blank lines; formatting and diff checks passed.
