# Desktop permissions

## Status

Planned. No implementation changes yet.

- [ ] Report computer-use readiness from silent macOS permission checks.
- [ ] Reject unavailable computer actions without triggering permission dialogs.
- [ ] Request permissions only through explicit desktop setup.
- [ ] Verify missing, granted, and revoked permissions.

## Scope

Devices currently advertise computer use unconditionally. Report it as available only when the
required permissions are granted. Check again before execution so revocation takes effect.
Keep shell access independent of desktop permissions.

Startup, discovery, chat, and rejected tool calls must not trigger permission dialogs.
Users should choose desktop setup before Agent requests access.

## Verification

Check startup, device discovery, and tool calls with permissions missing, granted, and revoked.
Confirm that ordinary chat and shell commands still work without desktop permissions.
Confirm that explicit setup can obtain permissions and enable computer use.

The unexpected Accessibility dialog reported on November 10 remains unexplained. Do not assume
this readiness bug caused it. See the [investigation log](../../../logs/2026-11-10-23-30-accessibility-prompt.md).
