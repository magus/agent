# Chat UI

## Status

Completed and verified on both Macs. See logs/2026-11-10-03-12-chat-ui.md for evidence.

- [x] Composer with model and reasoning controls.
- [x] Left/right messages without repeated role labels.
- [x] Compact, expandable tool rows.
- [x] Stable layout, scrolling, selection, and keyboard behavior.
- [x] Visual and functional checks, signed build, and installation on both Macs.

## Goal

Make Agent's chat comfortable for everyday use. Use the user's November 10 screenshots as the
visual reference. Keep GPUI and the existing central service architecture.

## Composer

- Use one rounded, dark composer with comfortable padding, subtle borders, and a growing text area
  with a maximum height. Put model, reasoning effort, and the send/stop icon in its bottom row.
- Move model selection out of the top toolbar. Show readable model names and the selected effort.
  Offer only reasoning levels supported by the selected model, including Automatic where supported.
- Apply selections to the current conversation's next turn and persist them. A running turn keeps
  its original settings. Model changes must resolve unsupported effort choices clearly.
- Keep defaults for future conversations in Conversation settings. Composer changes do not alter them.
- Preserve Enter to send, Shift+Enter for newline, Shift+Esc to focus, and typing from non-text
  controls. Preserve immediate message display, drafts, retry behavior, and the send/stop rules.
- Do not add attachment, voice, or approval controls just because the reference includes them.

## Conversation

- Put user messages in right-aligned bubbles. Put assistant messages on the left with a quieter
  treatment. Remove repeated You and Agent labels. Use restrained color and readable line lengths.
- Keep Markdown, code, links, selectable text, and copying working. Long content must wrap or scroll
  within its own bounds rather than widening the window.
- Keep enabled and disabled devices clearly visible by human-readable name. Do not bury access
  controls in the visual cleanup or add a default tool target.

## Tools

- Render each call as a compact, collapsed, single-line row with an icon, useful action summary,
  device name, status, and disclosure indicator. Truncate long summaries.
- Expand on demand to show the full command or action, arguments, output, errors, and screenshots.
  Keep the details selectable and screenshots inspectable.
- Make running, failed, stopped, and completed states distinct without relying only on color.
  Failed calls must remain obvious when collapsed.
- Preserve expansion state during updates. Do not add a model call to generate summaries.

## Layout and implementation

- Avoid transient labels that move surrounding content. Keep control dimensions stable across
  loading and execution states. User-requested expansion may change height; ordinary updates must
  not repeatedly collapse, expand, or move rows.
- Follow new content when the reader is at the bottom. Preserve their reading position when they
  scroll up. Do not reset the selection or draft during background updates.
- Split composer, message rendering, and tool rendering into clearly named components as useful.
  Keep mod.rs files limited to declarations and exports. Reuse existing GPUI components and icons.
- Preserve other uncommitted work. Backward compatibility is not a requirement. Preserve actual
  conversation history, device grants, and credentials.
- Do not expand this milestone into streaming transport, context management, scheduling, or a
  general redesign of setup and device windows.

## Verification

Iterate on screenshots and actual interaction, not compilation alone. Check empty and populated
chats, long drafts and replies, Markdown/code, many tool calls, screenshots, errors, and narrow and
wide windows. Check selection/copy, keyboard focus, send/stop, scrolling, and stable tool expansion.

Verify that model and effort selections reach the service and the next inference request, survive
reopening, and leave future-conversation defaults unchanged. Use a bounded live request and an
authorized harmless mini tool call. Do not send the user's draft or modify existing conversations
for test purposes.

Run formatting, Clippy, and relevant tests. Build, sign, notarize, install on both Macs, and verify
startup and shared chat access. Record results and representative screenshots in the delivery log.
