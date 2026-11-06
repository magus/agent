# agent-identity

## Context

The user chose `com.iamnoah.agent` and requested clear separation from Roomy's notarization naming.

## Decisions

- Use the final app identifier and derive the helper signing identifier from it.
- Require an explicit `AGENT_NOTARY_PROFILE` for notarization rather than defaulting to Roomy's
  profile or inventing a profile that does not exist. Leave Roomy's credentials and build untouched.
- Do not redeploy the mini or submit for notarization in this change. The new identity needs its
  own initial macOS permission grants when installed.

## Changes

- Updated bundle metadata, packaging help, and build documentation.
- Documented interactive creation of a neutral `apple-notary` profile as an example; no credentials
  or Keychain items were created, copied, renamed, or exposed.

## Verification

- Local signed build passed with strict nested signature verification.
- Verified the app identifier is `com.iamnoah.agent` and the helper identifier is
  `com.iamnoah.agent.desktop_service`.
- Shell syntax, ShellCheck, and `git diff --check` passed. No notarization submission was made.
