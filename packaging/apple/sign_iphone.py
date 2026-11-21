import datetime
import hashlib
import pathlib
import plistlib
import re
import shutil
import subprocess
import tempfile


def sign_iphone(app_dir):
    with (app_dir / "Info.plist").open("rb") as stream:
        bundle_id = plistlib.load(stream)["CFBundleIdentifier"]

    identity_output = subprocess.check_output(
        ["security", "find-identity", "-v", "-p", "codesigning"],
        text=True,
    )
    identity_set = set(re.findall(r"\b[0-9A-F]{40}\b", identity_output))
    profile_dir = pathlib.Path.home() / "Library/Developer/Xcode/UserData/Provisioning Profiles"
    profile_list = []

    for path in profile_dir.glob("*.mobileprovision"):
        result = subprocess.run(
            ["security", "cms", "-D", "-i", str(path)],
            capture_output=True,
            check=False,
        )
        if result.returncode:
            continue

        profile = plistlib.loads(result.stdout)
        identity = _matching_identity(profile, bundle_id, identity_set)
        if identity:
            profile_list.append((path, profile, identity))

    if not profile_list:
        raise SystemExit("No installed iPhone provisioning profile and signing identity match Agent.")

    # Prefer an Agent-specific profile, then the profile with the latest expiry.
    profile_list.sort(
        key=lambda item: (
            not item[1]["Entitlements"]["application-identifier"].endswith("*"),
            item[1]["ExpirationDate"],
        ),
        reverse=True,
    )
    profile_path, profile, identity = profile_list[0]
    source = profile["Entitlements"]
    application_id = profile["ApplicationIdentifierPrefix"][0] + "." + bundle_id
    entitlement_by_key = {
        "application-identifier": application_id,
        "com.apple.developer.team-identifier": source["com.apple.developer.team-identifier"],
        "keychain-access-groups": [application_id],
        "get-task-allow": source.get("get-task-allow", False),
    }

    shutil.copyfile(profile_path, app_dir / "embedded.mobileprovision")
    with tempfile.TemporaryDirectory(dir=app_dir.parent) as temporary_dir:
        entitlement_path = pathlib.Path(temporary_dir) / "iphone.entitlements"
        entitlement_path.write_bytes(plistlib.dumps(entitlement_by_key))
        subprocess.run(
            ["codesign", "--force", "--sign", identity, "--entitlements", str(entitlement_path), str(app_dir)],
            check=True,
        )


def _matching_identity(profile, bundle_id, identity_set):
    source = profile["Entitlements"]
    if source.get("com.apple.developer.team-identifier") != "78Z6KJV687":
        return None
    if profile["ExpirationDate"] <= datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None):
        return None
    if not profile.get("ProvisionedDevices") and not profile.get("ProvisionsAllDevices"):
        return None

    application_id = profile["ApplicationIdentifierPrefix"][0] + "." + bundle_id
    allowed_id = source["application-identifier"]
    matches_bundle = application_id == allowed_id or (
        allowed_id.endswith("*") and application_id.startswith(allowed_id[:-1])
    )
    if not matches_bundle:
        return None

    for certificate in profile["DeveloperCertificates"]:
        fingerprint = hashlib.sha1(certificate).hexdigest().upper()
        if fingerprint in identity_set:
            return fingerprint
    return None

