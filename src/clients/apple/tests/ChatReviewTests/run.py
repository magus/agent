"""Run the production chat views against a local bridge fixture."""

import argparse
import pathlib
import plistlib
import shutil
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("platform", choices=["macos", "simulator"])
    args = parser.parse_args()
    root = pathlib.Path(__file__).resolve().parents[5]
    build_dir = root / ".build"
    build_dir.mkdir(exist_ok=True)

    subprocess.run(["npm", "--prefix", "assets/markdown", "run", "build"], cwd=root, check=True)

    with tempfile.TemporaryDirectory(dir=build_dir) as stage:
        app = pathlib.Path(stage) / "ChatReviewTests.app"
        build_app(root=root, platform=args.platform, app=app)

        match args.platform:
            case "macos":
                executable = app / "Contents/MacOS/ChatReviewTests"
                for options, expected in [
                    ([], "Layout passed:"),
                    (["--startup-error"], "Visible error passed: Startup fixture failed"),
                    (["--connection-error"], "Visible error passed: Service fixture is offline"),
                    (["--find"], "Find reopened chat and typed into Search titles"),
                ]:
                    run_check([executable, *options], expected=expected)
            case "simulator":
                run_simulator_test(app)
            case _:
                raise ValueError(args.platform)


def build_app(*, root, platform, app):
    apple = root / "src/clients/apple"
    tests = apple / "tests/ChatReviewTests"
    source_list = sorted((apple / "shared").glob("*/*.swift"))
    match platform:
        case "macos":
            sdk = "macosx"
            target = "arm64-apple-macosx15.0"
            bundle_dir = app / "Contents"
            resource_dir = bundle_dir / "Resources"
            executable_dir = bundle_dir / "MacOS"
            template = root / "packaging/macos/Agent.app.template/Contents/Info.plist"
            source_list.extend([
                apple / "macos/ComposerTextInput.swift",
                apple / "macos/AgentCommands.swift",
                apple / "macos/DesktopRestartDelay.swift",
                tests / "DesktopRestartDelayTests.swift",
                tests / "FindAfterClose.swift",
            ])
        case "simulator":
            sdk = "iphonesimulator"
            target = "arm64-apple-ios18.0-simulator"
            bundle_dir = resource_dir = executable_dir = app
            template = root / "packaging/apple/Info.plist"
        case _:
            raise ValueError(platform)

    executable_dir.mkdir(parents=True)
    markdown_dir = resource_dir / "markdown"
    markdown_dir.mkdir(parents=True)
    info = plistlib.loads(template.read_bytes())
    info.update(
        CFBundleIdentifier="com.iamnoah.agent.chatreviewtests",
        CFBundleName="ChatReviewTests",
        CFBundleExecutable="ChatReviewTests",
        CFBundleShortVersionString="1.0",
        CFBundleVersion="1",
    )
    if platform == "simulator":
        info["CFBundleSupportedPlatforms"] = ["iPhoneSimulator"]

    plist_path = bundle_dir / "Info.plist"
    plist_path.write_bytes(plistlib.dumps(info))
    for filename in ["index.html", "style.css"]:
        shutil.copy2(root / "assets/markdown" / filename, markdown_dir)
    shutil.copy2(root / ".build/markdown/renderer.js", markdown_dir)
    shutil.copy2(tests / "checkLayout.js", resource_dir)

    source_list.append(tests / "ChatReviewTests.swift")
    subprocess.run(
        [
            "xcrun", "--sdk", sdk, "swiftc", "-parse-as-library", "-swift-version", "5",
            "-module-cache-path", root / ".build/module-cache", "-target", target,
            "-import-objc-header", apple / "shared/bridge/AgentBridge.h",
            *source_list, "-o", executable_dir / "ChatReviewTests",
        ],
        check=True,
    )
    subprocess.run(["codesign", "--force", "--sign", "-", app], check=True)


def run_simulator_test(app):
    bundle_id = "com.iamnoah.agent.chatreviewtests"
    subprocess.run(["xcrun", "simctl", "install", "booted", app], check=True)
    try:
        run_check(
            ["xcrun", "simctl", "launch", "--console", "booted", bundle_id],
            expected="Layout passed:",
        )
    finally:
        subprocess.run(["xcrun", "simctl", "uninstall", "booted", bundle_id], check=True)


def run_check(command, *, expected):
    result = subprocess.run(command, capture_output=True, text=True, timeout=45)
    print(result.stdout, end="")
    print(result.stderr, end="")
    result.check_returncode()
    if expected not in result.stdout:
        raise RuntimeError(f"Test ended before reporting {expected!r}")


if __name__ == "__main__":
    main()
