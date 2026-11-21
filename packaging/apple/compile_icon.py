import plistlib
import subprocess

import render_icon


def compile_icon(app_dir, *, platform):
    render_icon.render_icon()

    match platform:
        case "macos":
            asset_platform = "macosx"
            minimum_version = "15.0"
            resource_dir = app_dir / "Contents/Resources"
            plist_path = app_dir / "Contents/Info.plist"
            device_list = ["mac"]
        case "simulator" | "iphone":
            asset_platform = "iphonesimulator" if platform == "simulator" else "iphoneos"
            minimum_version = "18.0"
            resource_dir = app_dir
            plist_path = app_dir / "Info.plist"
            device_list = ["iphone", "ipad"]
        case _:
            raise ValueError(f"Unsupported icon platform: {platform}")

    resource_dir.mkdir(parents=True, exist_ok=True)
    metadata_path = app_dir.parent / "icon-info.plist"
    command = [
        "xcrun", "actool", "assets/Agent.icon",
        "--compile", str(resource_dir),
        "--platform", asset_platform,
        "--minimum-deployment-target", minimum_version,
        "--app-icon", "Agent",
        "--output-partial-info-plist", str(metadata_path),
    ]
    for device in device_list:
        command.extend(["--target-device", device])
    subprocess.run(command, check=True)

    info = plistlib.loads(plist_path.read_bytes())
    icon_info = plistlib.loads(metadata_path.read_bytes())
    info.update(icon_info)
    plist_path.write_bytes(plistlib.dumps(info))
