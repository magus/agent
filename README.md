# `agent`

A Mac and iPhone agent with Mac desktop control, written in
[rust](https://rust-lang.org/) and [swift](https://www.swift.org/).

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

Build the [native Mac or iPhone client](packaging/apple/README.md).

Run the Mac app

```sh
packaging/macos/app.sh
open dist/Agent.app
```

## permissions

1. Open **System Settings → Privacy & Security**.
2. Add `Agent.app` to **Accessibility** and **Screen & System Audio Recording** and enable both.
3. Quit and reopen `Agent.app`
4. Open **Agent → Desktop Permissions…**, choose **Check capture**, and confirm the preview.

Allow the additional screen-capture confirmation if macOS shows it when capturing.
Desktop control works whether the Mac is locked or unlocked. Each result reports the lock state.
It never unlocks the Mac. While unlocked, its input moves the pointer and focus of whoever is using it.

## devices

Open **Devices…** from the settings (gear) menu, or the Agent menu on the Mac, to pair devices and revoke access.
Pairing is directional and requires approval on the target.
See [direct tool calls](src/agent/devices/README.md) to run commands and desktop actions without a model.

## chat

Connect to a [central service](docs/central-service.md#startup), then sign in through
**ChatGPT account…** in the settings (gear) menu once. Every paired client shares that account and conversation history.
Choose a conversation and model, then send a task. **Devices** shows each device by name.
Enabled devices use full contrast; disabled devices are gray.
Click a device to change its access for the conversation. Each tool call names its device.
Chat works from any connected device, even when no devices are enabled for tools.

**Conversation settings…** in the settings (gear) menu (⌘, on the Mac) sets the model, reasoning level, and enabled devices for new
conversations started from this device. Automatic reasoning uses the model’s default. Existing conversations are unchanged.

Typing from non-text controls focuses the composer. Conversation text can be selected and copied. Shift+Esc focuses the composer.
Enter sends a message. Shift+Enter adds a newline. With an empty textbox during a task,
the send icon becomes Stop. Closing the app does not stop a task. After an interruption,
**Retrieve results** reads the original requests without repeating them.

## source

- `src/clients/apple/` owns the native Mac and iPhone interfaces.
- `src/agent/client/` owns shared service clients and message delivery.
- `src/agent/service/` owns execution, ChatGPT credentials, and conversation persistence.
- `src/agent/devices/` owns pairing, transport, and local tool execution.
- `src/agent/protocol/` owns shared message and conversation data.
- `src/agent/platform/` owns shared OS facilities.

The native launcher starts the interface or forwards commands to the independent Rust service binary.
The Swift interfaces use the shared Rust static library. The client and service do not import each
other's implementation.

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
