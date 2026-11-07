# app-windows-login-result

## Context

Keep main.rs focused on startup and make the browser login result clear. The permission and
ChatGPT windows remain separate.

## Decisions

- Put lifecycle and menus in AgentApp. Each setup view owns its window.
- Share activation and redraw behavior without adding a window framework.
- Show login success only after identity verification and Keychain storage finish.

## Changes

- main.rs now contains argument handling and startup. AgentApp.rs owns menus, lifecycle, and
  desktop-service supervision.
- PermissionSetup and ChatGptSetup own their window options and handles. window.rs shares window
  activation and redraw observation. Reopening a view activates its existing window.
- Added a self-contained HTML callback page with light and dark styles, clear success and failure
  states, and directions to return to Agent. No external assets or scripts are loaded.
- The callback waits for token validation and Keychain storage before showing success. A failed
  exchange shows a failure page. Closing the browser does not undo a saved login.

## Verification

- Formatting, Clippy, and diff checks passed.
- All five tests passed, including a loopback test that checks the response waits for a result,
  renders success and failure separately, and does not include the callback code. Its first run
  failed because the shell sandbox blocked binding a socket; the authorized rerun passed.
- Browser visual verification remains unperformed. The browser tool rejected the local preview
  file under its URL policy; no alternate route was used to bypass that restriction.
- The signed build passed notarization and ticket validation, submission
  `dfd6d287-8709-4251-a31f-5b1d941f5900`. Launched `dist/Agent.app` and confirmed the saved ChatGPT
  account still loads. The Set up Agent menu opens the other window and reuses it on a second click.
