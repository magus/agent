# Remote maintenance

## Status

Planned. Routine remote commands already work through Agent's paired device connection.
SSH-independent updates and recovery still need a defined workflow and verification.

- [x] Verify a MacBook-to-mini shell command through Agent without SSH.
- [ ] Make existing device commands easy to invoke from development tools without scratch scripts.
- [ ] Add authenticated file transfer for builds and diagnostic files
      ([file transfer plan](../file-transfer/README.md)).
- [ ] Stage and verify a signed app update before replacing the installed app.
- [ ] Run update and restart work outside the process being replaced.
- [ ] Reconnect and report the installed version and health; recover the prior build on failure.
- [ ] Verify updates preserve device grants, credentials, permissions, history, and request IDs.
- [ ] Prove the complete update flow with SSH unavailable.

## Existing behavior

Agent's paired TLS connection already supports shell commands, computer actions, and retrieving
results by request ID. These requests do not use Tailscale SSH. Tailscale supplies network access;
its SSH authorization is a separate mechanism.

On November 11, the native Mac client ran `hostname` on the mini through the existing device
connection. The command returned `Joahs-Mac-mini.local` with exit code 0. No SSH approval was used
for that request.

Use this connection for routine diagnostics and remote setup. The existing `--device-request`
command and local administration socket provide the transport. Improve their usability rather
than adding another remote command protocol.

## Update and recovery

Replacing Agent can stop the same process carrying the command and its result. An update needs a
small independent supervisor or updater that survives that replacement. Inspect existing launch
and supervision code before adding one. Keep this separate from the model loop.

Use the existing authenticated device connection to transfer files and authorize updates. Verify
the expected signing identity, stage files, and retain a recoverable previous build. Do not add
another pairing system or weaken directional access grants.

Keep updates scoped to explicitly authorized maintenance. A conversation's normal tool access must
not silently become a new permission to administer other devices.

If Agent and its supervisor are both unavailable, Agent cannot repair itself over its own stopped
connection. Retain SSH or local access as an explicit emergency recovery path. Routine development
must not depend on repeatedly approving Tailscale SSH.

## Scope

Track this separately from the [native UI migration](../native-ui/README.md). Use the existing shell
connection during that migration. Build the missing maintenance workflow as future work rather
than expanding the UI migration into an update-system rewrite.
