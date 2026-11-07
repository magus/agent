# `agent`

macOS desktop control written in [rust](https://rust-lang.org/) and [swift](https://www.swift.org/).

> [see how the desktop service works](src/desktop_service/README.md)

## setup

1. Install [rust](https://rust-lang.org/tools/install/)
2. Install [Xcode](https://developer.apple.com/xcode/)
3. Open Xcode and install macOS support when prompted
4. Install [Metal toolchain](https://developer.apple.com/documentation/xcode/downloading-and-installing-additional-xcode-components)

   ```sh
   sudo xcode-select --switch /Applications/Xcode.app/Contents/Developer
   xcodebuild -downloadComponent MetalToolchain
   ```

5. Initialize submodules after cloning

   ```sh
   git submodule update --init --recursive
   ```

6. Set up a Developer ID Application certificate in Keychain for
   [app signing](packaging/macos/README.md#signing-and-permissions)

## development

Run an unnotarized `Agent.app`

```sh
packaging/macos/app.sh
open dist/Agent.app
```

## permissions

1. Open **System Settings → Privacy & Security**.
2. Add `Agent.app` to **Accessibility** and **Screen & System Audio Recording** and enable both.
3. Quit and reopen `Agent.app`
4. Choose **Check capture** in Agent and confirm that the preview shows your app windows.

Allow the additional screen-capture confirmation if macOS shows it when capturing.
Lock the Mac before sending desktop-control requests; the current service rejects unlocked use.

## test

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## packaging

[packaging/macos/](packaging/macos/README.md)

## version

Install `cargo set-version` from [cargo-edit](https://github.com/killercup/cargo-edit)

```sh
cargo install cargo-edit --locked --no-default-features --features set-version
```

```sh
cargo set-version -p agent --bump patch --offline
cargo set-version -p agent --bump minor --offline
cargo set-version -p agent --bump major --offline
cargo set-version -p agent 0.1.0 --offline
cargo set-version -p agent --bump minor --offline --dry-run
```
