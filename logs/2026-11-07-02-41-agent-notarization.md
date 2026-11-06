# agent-notarization

## Context

The user created and validated the `apple-notary` Keychain profile and explicitly approved uploading
Agent.app to Apple's notarization service. This completes the previously untested packaging path.

## Decisions

- Use the default profile without environment overrides and the final `com.iamnoah.agent` identity.
- Keep the mini's installed proof unchanged.

## Changes

- Ran `packaging/macos/app.sh --notarize` to build, sign, submit, and staple the app.

## Verification

- Build and upload passed. Apple submission ID: `beff372a-25cb-4c8f-92fb-c7a040a56325`.
- Apple accepted the submission. Stapling and ticket validation passed.
- Final `dist/Agent.app` passed strict nested signature verification and Gatekeeper assessment
  with `source=Notarized Developer ID`. Its stapled ticket validated after moving out of staging.
- The command succeeded without environment overrides. Earlier profile-setup and notarization
  follow-ups are now resolved; permission onboarding remains separate work.
