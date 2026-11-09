# ChatGPT request errors

## Context

Agent shows a usage-limit message while Codex still works and the account has credits.
The client hid the actual stream error behind a generic usage-limit suggestion.

## Decisions

- Determine the actual provider error before attributing failure to credits.
- Preserve error codes and incomplete-response reasons without logging tokens or response bodies.
- Leave account spending controls unchanged. Enabling credits for other apps requires the user’s choice.

## Changes

- Stream failures now show their provider code. Incomplete responses show their reason.
- Only bounded code identifiers are displayed, never arbitrary provider messages or credentials.
- Installed the signed, notarized diagnostic update on the mini. The local app bundle is updated;
  the running desktop UI did not need a restart for this service-side error change.

## Verification

- Formatting, Clippy, and all 39 tests passed, including error detail and credential exclusion tests.
- A live central inference check returned `subscription_sharing_usage_limit_exceeded`.
- Read ChatGPT Settings → Usage. Weekly allowance was exhausted, credits remained, and
  “Allow other apps to use credits after reaching your usage limit” was disabled.
- Both Agent registrations allowed 100% plan usage. No account settings were changed.
- OpenAI documents that this error can reflect an app-specific usage limit and does not establish
  that the account has no credits. The observed credit-sharing setting explains this failure.
- Notarization accepted submission `3a546be8-bde2-4ea6-969f-54d41a426302`.

## Follow-ups

- User can enable credit use for other apps in ChatGPT Settings → Usage, then retry Agent.
  The settings tab is left open. This setting applies to other authorized apps, not only Agent.
