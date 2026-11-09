# Conversation defaults

## Context

Model and reasoning defaults belong beside device defaults in Conversation settings.

## Decisions

- Store defaults centrally for each starting device. Copy them when creating a conversation.
- Keep existing conversations unchanged and preserve older stored data.
- Automatic reasoning keeps the provider's default. Show only levels from its model catalog.
- Changing a conversation's model resets reasoning to Automatic to avoid an unsupported level.

## Changes

- Added model and reasoning dropdowns to `app/chat/conversation_settings.rs`.
- Added shared model settings and model options in `protocol/`.
- The service stores defaults separately from device grants and copies them into creation records.
  Creation retries retain the original settings. The loop passes saved reasoning to inference.
- Updated the root README. Installed the signed, notarized build on both Macs.

## Verification

- Formatting, Clippy, dependency rules, and all 37 tests passed.
- Tests cover restart persistence, creation retries, and unchanged existing conversations.
- Used the UI to select `gpt-6-luna` and high reasoning. A new UI-created conversation inherited
  both and completed a live inference request with `OK`.
- Both Macs read the same conversation settings. Existing history and device grants were unchanged.
- Restored `gpt-6.1-sol` and Automatic reasoning. Restarted the service and confirmed both defaults
  and the test conversation's distinct settings survived.
- Signing and notarization passed. Submission `6ea28eed-f890-4edb-ac44-4aa638664bd6` was accepted.
