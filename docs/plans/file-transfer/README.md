# File transfer

## Status

Planned, not started. Moving files between devices today means Tailscale SSH plus `scp`, or serving
a file on the tailnet temporarily and downloading it. Both are awkward, and plain `scp` has dropped
executable bits on copied binaries (see `logs/2026-11-07-04-00-permission-setup.md`).

- [ ] Add a `copy` operation to the paired device connection, with a device and path on both ends.
- [ ] Send files in chunks and verify each transfer with a SHA-256 hash.
- [ ] Preserve file mode, including the executable bit, and support directories.
- [ ] Keep a free-space reserve on the destination (5 GB by default).
- [ ] Show each transfer in the conversation and history like a shell call.
- [ ] Add a development CLI command next to `--device` / `--service-device`.
- [ ] Add a model tool that uses the same operation.
- [ ] Verify a transfer larger than 64 MiB, an executable, and a directory, in every direction
      (upload, download, device to device), without SSH.

## Design

Use Agent's existing paired TLS device connection (`RemoteRequest` in
`src/agent/devices/device_service/remote_request.rs`). Do not add another protocol, pairing system,
or network path. This also covers the file transfer step in the
[remote maintenance plan](../remote-maintenance/README.md), whose update staging can build on it.

### API

One symmetric `copy` operation. Both ends name a device and a path:

```json
{
  "source": { "device": "macbook", "path": "/Users/noah/build/Agent.app" },
  "destination": {
    "device": "mini",
    "path": "/Users/noah/Applications/Agent.app"
  },
  "overwrite": false
}
```

- The same call covers upload, download, and device to device. Nothing assumes the caller is one
  of the two ends.
- `"device": "local"`, or leaving `device` out, means the machine running the call.
- An existing destination is refused unless `overwrite` is `true`.
- The CLI and model tool may add conveniences, such as a destination ending in `/` meaning "into
  this folder". The operation itself takes exact paths.
- The result reports the final path, size, file count, and SHA-256 of what was written.

For device to device, the caller relays: it reads chunks from the source and writes them to the
destination over connections it already has. That keeps directional grants unchanged; the two
devices need no grant to each other. Consider a direct path later only if relaying is too slow.

### Permissions

Shell access already gives full filesystem access, so a separate gate for transfers would add no
real protection. A transfer touches two machines, so the rule is: **the caller needs shell-level
access on both the source and the destination device.** Use the same directional device grants as
the shell tool. Add no directory restriction or extra approval beyond that.

Devices without a shell, such as the iPhone client or the central service, can still be either end.
On those devices, paths resolve inside a dedicated transfers folder in the app's own container, not
anywhere on disk.

The CLI is run by a person on the local machine, so its local end needs no grant.

### Chunking and integrity

Each device message is limited to 64 MiB (`src/agent/devices/transport.rs`), and app builds can be
larger. Do not raise the limit. Send files as a sequence of bounded chunks and check a SHA-256 hash
of the whole transfer at the end. Messages are JSON today, so chunk contents need an encoding
(base64 costs about a third more) unless a binary frame is added. Decide that when implementing.

Write into a temporary file beside the destination and rename it into place only after the hash
matches, so a failed or interrupted transfer never leaves a partial file at the destination.

### Metadata and directories

Preserve file mode, especially the executable bit. Support directories, either as an archive (tar)
or as a list of entries with their own modes. Copying an app bundle should give a runnable result
without manual fixes.

### Free disk space

Never come close to filling a disk. Before starting, check that the destination will keep at least
a reserve of free space after the transfer: 5 GB by default, configurable. Refuse the transfer up
front if it would not. Check again while writing in case other processes use space meanwhile, and
stop and clean up the temporary file if the reserve would be crossed.

### Visibility

Transfers must be as visible as the shell commands they replace. Each transfer appears in the
conversation and its history like a shell call, showing source and destination (device and path),
size, file count, SHA-256, and outcome, including refusals and failures.

### Two ways in

- A CLI command for development work, beside the existing `--device` and `--service-device`
  request commands, so routine transfers don't need scratch scripts or SSH.
- A model tool that calls the same operation, available wherever the shell tool is.

Both use one implementation of the operation.
