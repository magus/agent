# Native Apple builds

Build scripts use Python 3. Install Node.js and npm for the bundled Words message renderer. Builds use the pinned dependencies
in `assets/markdown/package-lock.json`; the installed app does not need Node.js or network assets.

## Mac

```sh
packaging/apple/build.py macos
open .build/native/macos/Agent.app
```

Sign, notarize, and staple using the existing `apple-notary` Keychain profile

```sh
packaging/apple/build.py macos --notarize
```

The bundle contains the native interface, `agent_service`, and `desktop_service`. It keeps
`com.iamnoah.agent` and the existing Developer ID identity. The service commands remain available
through `Agent.app/Contents/MacOS/agent --help`.

## Renderer tests

```sh
npm --prefix assets/markdown ci --ignore-scripts
npm --prefix assets/markdown test
```

These check formatting, unsafe content, and incremental parsing. Test selection, scrolling, and
appearance in the native apps as well.

## iPhone Simulator

Install iOS support in Xcode, then build

```sh
rustup target add aarch64-apple-ios-sim
packaging/apple/build.py simulator
xcrun simctl install booted .build/native/simulator/Agent.app
xcrun simctl launch booted com.iamnoah.agent
```

Use Xcode's Device Hub to open and interact with the simulated iPhone. Pair this client with the
central service. It shares central credentials and conversations but exposes no execution tools.

## iPhone

Use an installed provisioning profile for Agent or a wildcard profile from the existing Apple team.
The profile must include the phone and have a matching signing certificate in Keychain.

```sh
rustup target add aarch64-apple-ios
packaging/apple/build.py iphone
xcrun devicectl device install app --device <device-id> .build/native/iphone/Agent.app
xcrun devicectl device process launch --device <device-id> com.iamnoah.agent
```

The build selects a matching installed profile. Connect and unlock the phone, and enable Developer
Mode before installation. Simulator checks do not establish physical-device behavior.

Remaining native UI checks are listed in [TODO.md](../../TODO.md).
`packaging/macos/app.sh` builds the same native app and places it in `dist/Agent.app`.

## App icon

`assets/Agent.icon` is the shared Icon Composer document. The build uses Xcode's `actool` to generate
platform icons and merge their metadata into the bundle. Apple applies the mask and generates the
legacy Mac `.icns` fallback.

`assets/Agent.icon/Assets/agent-icon-square.svg` is the app-icon source of truth. Edit that file,
not the adjacent PNG. Every build regenerates the 1024-pixel PNG before compiling the icon.
The generated PNG is checked in so the document also opens directly in Icon Composer.

To regenerate it without building the app

```sh
python3 packaging/apple/render_icon.py
```

The export uses resvg 0.45.1, installed through Cargo into `.build/tools/` on first use. Later
exports reuse that pinned tool. Python runs the export; no Node.js wrapper is needed.
We use a raster layer because importing the SVG directly into Icon Composer loses filter softness.
`assets/agent-icon.svg` is the original standalone rounded artwork, not the app build input.
