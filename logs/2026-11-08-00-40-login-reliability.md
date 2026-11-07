# login-reliability

## Context

Finish cancellation and verify saved-session refresh, sign-out, and app focus after login.

## Decisions

Refresh must stay in the background. Keep credentials out of logs. Use the existing account only
for authorized checks and avoid unnecessary reauthorization.

Pending login must not expire inside Agent. Keep its listener until completion, cancellation, or
app exit. Keep individual socket and network timeouts so a stalled connection cannot hang the app.

## Changes

- Added credential-free per-attempt timings at the user's request. Record callback receipt, token
  exchange, identity validation, Keychain save, browser reply, worker completion, UI readiness,
  focus request, window activation, and the first enabled-button render. Rendering is not proof of
  physical screen presentation; compare it with the visible result.
- Removed the five-minute callback deadline and its UI text after a delayed approval reached a
  closed localhost listener. Agent now keeps waiting until the attempt completes or is cancelled.
- Fixed stale background results by scheduling native display on the next event-loop turn in
  `src/agent/app/core/window.rs`. Merely marking the layer dirty did not reliably redraw an inactive
  window. Marking it dirty during display also caused a spinner redraw loop; that approach was removed.
- Accepted callback sockets explicitly use blocking reads with a two-second timeout. macOS inherited
  the listener's nonblocking mode, which could discard an early browser connection.
- Confirmed unusable refresh tokens now clear the local session and show a sign-in message. Temporary
  errors preserve credentials. Sign-out retries revocation once before clearing local tokens.
- Added cancellation and refresh-error regression tests. Removed temporary diagnostics and the
  forced-refresh condition after the live proof.
- Moved login activation outside the entity update and explicitly redraw on activation. An attempt
  to wait for the next display frame gave an inconclusive focus result and was removed. The user
  had seen the focused window keep its disabled button even though login had finished.

## Verification

- Eight tests passed. The existing callback test initially timed out, exposed the socket bug, and
  passed after the fix. Final formatting, Clippy, and whitespace checks passed.
- Live cancellation restores enabled controls and shows "Login cancelled" without restarting.
- Forced one normal refresh in a temporary signed build. It saved replacement tokens and completed
  the model request with `1989`, without opening a browser or using an API key.
- Live sign-out confirmed revocation and removed the local tokens. The signed-out state survived a
  restart. Reauthorization restored the saved account, but the first focus check exposed the redraw
  loop. The corrected build passed another live cancellation check.
- Final `dist/Agent.app` was signed, notarized, and stapled with `apple-notary`.
- Reopened login at the user's request. Approval completed, the callback showed "Agent connected",
  and Agent updated to "Signed in with ChatGPT" without restarting.
- The user confirmed Agent came forward immediately. The remaining issue is the visible delay
  before the button becomes enabled. Clippy passed for the activation-order change.
- The activation-order build passed signing, notarization, and stapling. It is running locally
  with a fresh login open for the user's comparison.

- Timed live login completed. Relative to callback receipt, token exchange finished at 538 ms,
  identity validation at 922 ms, Keychain save and browser reply at 955 ms, worker completion at
  957 ms, UI readiness and focus request at 964 ms, and enabled-button rendering at 971 ms.
  The first window-activation event arrived at 5723 ms. This measures a 4759 ms gap between the
  focus request and activation notification, not a delayed UI state update.
- The user confirmed the button was already back to "Continue with ChatGPT" when Agent appeared
  in the timed run. The render hook does not measure physical presentation; this observation supplies
  the visible confirmation. Keep the hooks for future reports rather than claiming every run is fast.
- The no-timeout callback change passed all eight tests. The timing build passed Clippy.
