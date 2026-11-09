# Chat responsiveness

## Context

A new conversation clears the selected model and disables Send. Sending in existing conversations
also gives delayed feedback. The previous live checks did not cover creating a new conversation.

## Decisions

- Avoid transient status rows that move conversation content. Remove the sending indicator entirely;
  show delivery status and Retry only after a failure. Keep newer drafts untouched.

Preserve a valid model choice for new conversations. Show messages locally before network submission.
Keep delivery state separate from confirmed history, and match acknowledgements by message ID.

## Changes

- Keep the selected available model when a new conversation has no saved model. Fall back to the
  first available model only when needed.
- Refresh conversation selection immediately. Show Sending while the request is in flight and
  add the acknowledged message directly to the client view without waiting for the next poll.
- Pin submissions to their original service and conversation. Clear the composer on Enter and keep
  it editable. Acknowledgements never clear a newer draft.
- Keep unconfirmed messages visible across polls. Show Retry after failure and reuse the original
  message receipt to prevent duplicate execution. This replaces the earlier acknowledgement-first UI.
- Discard older poll results when a newer cache update has already been applied.

## Verification

- Service catalog round trips measured 522–572 ms before the change. Sending previously added a
  polling delay after acceptance.
- All 36 tests passed, including model selection for empty new conversations. Formatting, Clippy,
  and diff checks passed.
- Restored and sent the user's original question. It received the expected two-device reply.
- Created another conversation in the installed UI, kept the selected model without refreshing,
  sent with Enter, and received OK. The draft cleared and Stop appeared after 681 ms in that run.
- Returned the UI to the user's original conversation. Installed the notarized build on both Macs.
  Apple accepted submission `befa7857-93c3-4139-80c4-96a15e6b4d28`.

- Immediate-send follow-up passes all 38 tests, formatting, and Clippy. The offline UI test showed the message and cleared composer without any service acknowledgement.
- Restored the service and used Retry. The service accepted one user entry. The response check failed because ChatGPT returned
  “The ChatGPT request failed or reached a usage limit. Check ChatGPT Settings → Usage.”
  No successful model reply is claimed for this test.
  A newer unsent draft remained intact through retry and completion.
- Installed the final notarized update on both Macs. Submission
  `0f276d95-6e35-4946-8c82-c91fec4ef94d` was accepted.

- Removed the transient sending row on November 10. Pending and confirmed messages render with
  the same layout; only failed delivery adds a compact status and Retry row. Composer logic is unchanged.
- Formatting, Clippy, signing, and notarization passed for this UI-only adjustment. Installed on both
  Macs. Apple accepted submission `95b20511-1817-46f0-a191-16cdb75a2d6d`.
