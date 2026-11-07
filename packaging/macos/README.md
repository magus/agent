# macOS packaging

Run from the repository root. Requires macOS 15+, Xcode with the Metal toolchain, Rust/Cargo,
and a usable Developer ID Application certificate in Keychain.

```sh
./packaging/macos/app.sh
open dist/Agent.app
```

The script compiles Rust and Swift for the host architecture with a macOS 15 deployment target.
It copies `Agent.app.template`, fills in the Cargo package version, and signs the bundle.
The template's executable symlinks point into `.build/`; copying replaces them with actual files.
The previous `dist/Agent.app` is replaced only after the requested checks pass.

## Signing and permissions

The script selects the sole Developer ID Application certificate. If several are available,
set `AGENT_SIGNING_IDENTITY` to the chosen name. Ad-hoc signing is not supported.

The app identifier is `com.iamnoah.agent`; the helper uses `com.iamnoah.agent.desktop_service`.
Keep these identifiers and the signing identity stable across builds. Install and launch one app
copy at a stable location before granting permissions. Building does not update an installed copy.

Launch through `open` so macOS attributes permission use to Agent.app.
See [service commands](../../src/agent/app/desktop_service/README.md) for runtime and permission diagnostics.

## Notarization

```sh
./packaging/macos/app.sh --notarize
```

This submits the app to Apple, staples the approval ticket, and verifies the result.
Without `--notarize`, the build is signed only. Notarization does not grant desktop permissions.

The default Keychain profile is `apple-notary`. Create it once if needed.

```sh
xcrun notarytool store-credentials apple-notary
```

The command prompts for credentials. `AGENT_NOTARY_PROFILE` is an optional override.
