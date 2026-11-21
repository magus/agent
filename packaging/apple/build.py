#!/usr/bin/env python3
import argparse
import os
import pathlib
import plistlib
import re
import shutil
import subprocess
import tempfile

import sign_iphone


def main():
    parser = argparse.ArgumentParser(description="Build the native Agent app.")
    parser.add_argument(
        "platform", nargs="?", default="macos",
        choices=["macos", "iphone", "simulator", "simulator-proof"],
    )
    parser.add_argument("--notarize", action="store_true")
    parser.add_argument("--dist", action="store_true", help="Also copy the Mac app to dist/Agent.app.")
    args = parser.parse_args()
    if (args.notarize or args.dist) and args.platform != "macos":
        parser.error("--notarize and --dist apply only to Mac builds")

    repo_dir = pathlib.Path(__file__).resolve().parents[2]
    os.chdir(repo_dir)
    build(args, repo_dir=repo_dir)


def build(args, *, repo_dir):
    proof = args.platform == "simulator-proof"
    platform = "simulator" if proof else args.platform
    match platform:
        case "macos":
            rust_target = "aarch64-apple-darwin"
            swift_target = "arm64-apple-macosx15.0"
            sdk = "macosx"
            deployment = {"MACOSX_DEPLOYMENT_TARGET": "15.0"}
        case "simulator":
            rust_target = "aarch64-apple-ios-sim"
            swift_target = "arm64-apple-ios18.0-simulator"
            sdk = "iphonesimulator"
            deployment = {"IPHONEOS_DEPLOYMENT_TARGET": "18.0"}
        case "iphone":
            rust_target = "aarch64-apple-ios"
            swift_target = "arm64-apple-ios18.0"
            sdk = "iphoneos"
            deployment = {"IPHONEOS_DEPLOYMENT_TARGET": "18.0"}
        case _:
            raise ValueError(f"Unsupported platform: {platform}")

    cargo_dir = repo_dir / ".build/cargo"
    sdk_path = subprocess.check_output(["xcrun", "--sdk", sdk, "--show-sdk-path"], text=True).strip()
    build_env = os.environ | deployment | {"CARGO_TARGET_DIR": str(cargo_dir), "SDKROOT": sdk_path}
    profile = os.environ.get("AGENT_NOTARY_PROFILE", "apple-notary")
    if args.notarize:
        subprocess.run(
            ["xcrun", "notarytool", "history", "--keychain-profile", profile],
            check=True, stdout=subprocess.DEVNULL,
        )

    subprocess.run(["cargo", "build", "--locked", "--lib", "--target", rust_target], check=True, env=build_env)
    if platform == "macos":
        subprocess.run(
            ["cargo", "build", "--locked", "--bin", "agent", "--target", rust_target],
            check=True, env=build_env,
        )

    output_dir = repo_dir / ".build/native" / platform
    output_dir.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="build.", dir=output_dir) as stage:
        app_dir = pathlib.Path(stage) / "Agent.app"
        executable_dir = prepare_bundle(app_dir, platform=platform)
        build_renderer(app_dir, platform=platform)
        compile_app(
            executable_dir, platform=platform, proof=proof, sdk_path=sdk_path,
            swift_target=swift_target, cargo_dir=cargo_dir, rust_target=rust_target,
            build_env=build_env,
        )
        if platform == "macos":
            sign_mac(app_dir, executable_dir=executable_dir)
        elif platform == "iphone":
            sign_iphone.sign_iphone(app_dir)
        else:
            subprocess.run(["codesign", "--force", "--sign", "-", str(app_dir)], check=True)
        subprocess.run(["codesign", "--verify", "--deep", "--strict", str(app_dir)], check=True)

        if args.notarize:
            notarize(app_dir, profile=profile)

        # Replace the previous build only after signing and requested checks pass.
        destination = output_dir / "Agent.app"
        if destination.exists():
            shutil.rmtree(destination)
        app_dir.rename(destination)
    print(destination, flush=True)

    if args.dist:
        publish_dist(destination, repo_dir=repo_dir)


def prepare_bundle(app_dir, *, platform):
    if platform == "macos":
        executable_dir = app_dir / "Contents/MacOS"
        template = pathlib.Path("packaging/macos/Agent.app.template/Contents/Info.plist")
        plist_path = app_dir / "Contents/Info.plist"
    else:
        executable_dir = app_dir
        template = pathlib.Path("packaging/apple/Info.plist")
        plist_path = app_dir / "Info.plist"
    executable_dir.mkdir(parents=True)
    info = plistlib.loads(template.read_bytes())
    if platform == "macos":
        package_id = subprocess.check_output(["cargo", "pkgid", "--locked"], text=True).strip()
        version = package_id.rsplit("#", 1)[-1].rsplit("@", 1)[-1]
        info["CFBundleShortVersionString"] = version
        info["CFBundleVersion"] = version
    else:
        info["CFBundleSupportedPlatforms"] = ["iPhoneOS" if platform == "iphone" else "iPhoneSimulator"]
    plist_path.write_bytes(plistlib.dumps(info))
    return executable_dir


def build_renderer(app_dir, *, platform):
    subprocess.run(
        ["npm", "--prefix", "assets/markdown", "ci", "--ignore-scripts", "--no-audit", "--no-fund"],
        check=True,
    )
    subprocess.run(["npm", "--prefix", "assets/markdown", "run", "build"], check=True)
    resource_dir = app_dir / ("Contents/Resources/markdown" if platform == "macos" else "markdown")
    resource_dir.mkdir(parents=True)
    for source in ["assets/markdown/index.html", "assets/markdown/style.css", ".build/markdown/renderer.js"]:
        shutil.copy2(source, resource_dir)
    for package, filename, output in [
        ("words", "LICENSE.md", "Words-LICENSE.md"),
        ("words", "NOTICE.md", "Words-NOTICE.md"),
        ("highlights", "NOTICE.md", "Highlights-NOTICE.md"),
    ]:
        source = pathlib.Path("assets/markdown/node_modules/@pierre") / package / filename
        shutil.copy2(source, resource_dir / output)


def compile_app(
    executable_dir,
    *,
    platform,
    proof,
    sdk_path,
    swift_target,
    cargo_dir,
    rust_target,
    build_env,
):
    source_dir = pathlib.Path("src/clients/apple")
    source_list = []
    for feature in ["bridge", "chat", "chatgpt", "devices"]:
        source_list.extend(sorted((source_dir / "shared" / feature).glob("*.swift")))
    if platform == "macos":
        source_list.extend(sorted((source_dir / "macos").glob("*.swift")))
    elif proof:
        source_list.append(source_dir / "tests/NativeClientProof.swift")
    else:
        source_list.extend(sorted((source_dir / "ios").glob("*.swift")))

    command = [
        "swiftc", "-swift-version", "5", "-parse-as-library", "-sdk", sdk_path,
        "-target", swift_target, "-module-cache-path", ".build/module-cache",
        "-import-objc-header", str(source_dir / "shared/bridge/AgentBridge.h"),
        *map(str, source_list), "-L", str(cargo_dir / rust_target / "debug"), "-lagent_client",
        "-framework", "Security", "-framework", "SystemConfiguration", "-framework", "CoreFoundation",
    ]
    if platform == "simulator":
        command.extend([
            "-Xlinker", "-sectcreate", "-Xlinker", "__TEXT", "-Xlinker", "__entitlements",
            "-Xlinker", "packaging/apple/simulator.entitlements",
        ])
    command.extend(["-o", str(executable_dir / "agent")])
    subprocess.run(command, check=True, env=build_env)
    if platform == "macos":
        shutil.copy2(cargo_dir / rust_target / "debug/agent", executable_dir / "agent_service")
        helper_source_list = sorted(pathlib.Path("src/desktop_service/swift").glob("*.swift"))
        subprocess.run(
            ["swiftc", "-O", "-target", swift_target, "-module-cache-path", ".build/module-cache",
             *map(str, helper_source_list), "-o", str(executable_dir / "desktop_service")],
            check=True, env=build_env,
        )


def sign_mac(app_dir, *, executable_dir):
    identity = os.environ.get("AGENT_SIGNING_IDENTITY")
    if identity is None:
        output = subprocess.check_output(["security", "find-identity", "-v", "-p", "codesigning"], text=True)
        identity_list = re.findall(r'"(Developer ID Application: [^"]*)"', output)
        if len(identity_list) != 1:
            raise SystemExit("Expected one Developer ID Application identity")
        identity = identity_list[0]
    if not identity.startswith("Developer ID Application: "):
        raise SystemExit("Expected a Developer ID Application identity")

    command = ["codesign", "--force", "--options", "runtime", "--timestamp", "--sign", identity]
    for filename, identifier in [
        ("desktop_service", "com.iamnoah.agent.desktop_service"),
        ("agent_service", "com.iamnoah.agent"),
    ]:
        subprocess.run(command + ["--identifier", identifier, str(executable_dir / filename)], check=True)
    subprocess.run(command + [str(app_dir)], check=True)


def notarize(app_dir, *, profile):
    archive = app_dir.parent / "Agent.zip"
    subprocess.run(["ditto", "-c", "-k", "--keepParent", str(app_dir), str(archive)], check=True)
    subprocess.run(
        ["xcrun", "notarytool", "submit", str(archive), "--keychain-profile", profile, "--wait"],
        check=True,
    )
    subprocess.run(["xcrun", "stapler", "staple", str(app_dir)], check=True)
    subprocess.run(["xcrun", "stapler", "validate", str(app_dir)], check=True)
    subprocess.run(["spctl", "--assess", "--type", "execute", str(app_dir)], check=True)


def publish_dist(app_dir, *, repo_dir):
    dist_dir = repo_dir / "dist"
    dist_dir.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="build.", dir=dist_dir) as stage:
        staged_app = pathlib.Path(stage) / "Agent.app"
        subprocess.run(["ditto", str(app_dir), str(staged_app)], check=True)
        destination = dist_dir / "Agent.app"
        if destination.exists():
            shutil.rmtree(destination)
        staged_app.rename(destination)
    print(destination, flush=True)


if __name__ == "__main__":
    main()
