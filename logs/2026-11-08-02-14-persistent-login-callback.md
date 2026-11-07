# persistent-login-callback

## Context

Late callbacks after cancellation currently reach a closed port. Keep a local callback server from
the first login until Agent quits, and show the result of each attempt without reusing credentials.

## Decisions

Bind only to loopback. Block while idle instead of polling. Retain attempt states for the process
lifetime. Cancelled and completed attempts cannot deliver another authorization code.

Use one clickable login button. Pending text changes to "Cancel login" on hover, and clicking
cancels. Remove the separate Cancel button. GPUI's loading mode disables interaction, so do not
use it for this cancellable button.

## Changes

- Added `callback_server.rs` with a shared loopback listener and separate attempt states.
- Late and repeated callbacks return a result page. They cannot exchange a code again or affect a
  later login. Cancelling releases the waiting task while keeping the listener available.
- Replaced the separate cancel button with a clickable pending button. Hover changes its label to
  "Cancel login" and its icon to an X. Button width stays stable.

## Verification

- All 13 Rust tests, Clippy, and formatting checks passed. Socket tests cover cancellation races,
  late callbacks, duplicate callbacks, browser denial, malformed requests, and server shutdown.
- Built, notarized, and opened `dist/Agent.app`. Apple accepted the build and staple validation passed.
- In the running app, the pending button stayed enabled and clicking it returned to
  "Continue with ChatGPT" with "Login cancelled." The saved account remained connected.
- Hover changed the accessibility label to "Cancel login". The non-hover screenshot showed
  "Waiting for ChatGPT…". No separate cancel button remains.
