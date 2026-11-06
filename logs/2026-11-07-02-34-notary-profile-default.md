# notary-profile-default

## Context

The user rejected a required environment variable and approved a neutral `apple-notary` profile name.

## Decisions

- Default to `apple-notary`; keep `AGENT_NOTARY_PROFILE` as an optional override.
- Preserve Roomy's existing profile so its builds continue to work.
- Use supported notarytool credential setup. Do not expose or write saved secrets into files.

## Changes

- Updated packaging help, defaults, and setup documentation.

## Verification

- The existing `roomy-notary` profile authenticated successfully with a read-only history request.
- notarytool provides store-credentials but no rename/copy command. Metadata-only Keychain
  inspection did not locate an accessible credential item to alias. No credential values were read.
- The neutral profile has not yet been created or validated.

## Follow-ups

- Create `apple-notary` through interactive `xcrun notarytool store-credentials apple-notary`.
  This is one-time setup, not a required environment variable on each build.
