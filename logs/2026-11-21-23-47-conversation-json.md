# conversation json viewer

## Context

Noah wants to inspect the selected chat, including values the chat UI omits. He requested a larger viewer,
readable nested JSON, and an explanation of the missing search row and historical events.

## Decisions

- Fetch saved JSON on demand and record received OpenAI stream events separately for new turns.
- Include the request ID and whether each event belongs to chat or hosted search.

## Changes

- Added a conversation debug menu and a read-only JSON viewer.
- Added a renderer picker for native text and Pierre Words. Both display the currently selected
  JSON, and switching renderers does not fetch the document again.

## Verification

- Built and reopened the signed Mac client and deployed the service through the paired connection.
- Read the saved AAPL query and its $344.87 tool result through the viewer endpoint.
- The new “Conversation JSON viewer check” chat captured 92 events across three requests,
  including the hosted search call. The older AAPL chat has no historical event capture.
- All 53 Rust tests passed, as did Clippy and diff checks. Native UI inspection returned
  AXError.cannotComplete, so the menu was not visually verified.
- The Mac extension now opens a resizable 1280 × 900 conversation window. Viewers can toggle
  decoding JSON strings in tool arguments and outputs. Raw inspection keeps their string types.
- Search queries now appear in the timeline. Confirmed the deployed mini returned the original
  AAPL chat in question, search, answer order. It has zero raw events. The newer verification
  chat still has 92 events. Built and reopened the Mac client. All 55 tests and Clippy passed.
  Native window inspection still returned AXError.cannotComplete.
- Made Pierre Words the default JSON renderer. A native WebKit check with 5,000 JSON lines
  measured a 500-point viewport, 95,080-point document, successful internal scrolling, and 2×
  display scale. The old JSON view expanded the web view to the full document height.
- Added expandable search rows with citations first, then URLs grouped by site with favicons.
  The service reads metadata on demand from the original tool result or matching hosted call IDs
  in the event log. Verified both AAPL chats on the mini; the older one has 11 domains including
  the cited Yahoo Finance page, and the newer one has 13 including the cited Twelve Data page.
- All 55 Rust tests and Clippy passed. Rebuilt and reopened the signed Mac app.
