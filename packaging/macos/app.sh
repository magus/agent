#!/bin/sh

set -euo pipefail

stderr() {
    printf '%s\n' "$@" >&2
}

usage() {
    stderr "Usage: packaging/macos/app.sh [options]"
    stderr ""
    stderr "Build and sign dist/Agent.app for this Mac's architecture"
    stderr ""
    stderr "Options:"
    stderr "  --notarize   Submit to Apple and staple the distribution ticket"
    stderr "  -h, --help   Print help"
    stderr ""
    stderr "Environment:"
    stderr "  AGENT_SIGNING_IDENTITY   Certificate [default: sole Developer ID Application identity]"
    stderr "  AGENT_NOTARY_PROFILE     Saved credentials [default: apple-notary]"
}

# -----------------------------------------------------------------------------
# arguments
# -----------------------------------------------------------------------------

notarize=false

while [ "$#" -gt 0 ]; do
    case "$1" in
        --notarize) notarize=true; shift ;;
        -h|--help) usage; exit 0 ;;
        --) shift; break ;;
        *) usage; stderr "Unknown argument: $1"; exit 2 ;;
    esac
done

if [ "$#" -ne 0 ]; then
    usage
    exit 2
fi

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
PKG_MACOS="$REPO_ROOT/packaging/macos"
cd "$REPO_ROOT"

# -----------------------------------------------------------------------------
# signing identity
# -----------------------------------------------------------------------------

identity_list="$(security find-identity -v -p codesigning)"
signing_identity="${AGENT_SIGNING_IDENTITY:-}"

if [ -z "$signing_identity" ]; then
    developer_identity_list="$(printf '%s\n' "$identity_list" | sed -n 's/.*"\(Developer ID Application: [^"]*\)".*/\1/p')"
    identity_count="$(printf '%s\n' "$developer_identity_list" | awk 'NF { count++ } END { print count + 0 }')"

    if [ "$identity_count" -ne 1 ]; then
        stderr "Expected one usable Developer ID Application identity; found $identity_count."
        stderr "Set AGENT_SIGNING_IDENTITY if multiple identities are available."
        exit 1
    fi

    signing_identity=$developer_identity_list
fi

case "$signing_identity" in
    "Developer ID Application: "*) ;;
    *) stderr "A Developer ID Application identity is required; ad-hoc signing is not supported."; exit 1 ;;
esac

if ! printf '%s\n' "$identity_list" | grep -F -- "\"$signing_identity\"" >/dev/null; then
    stderr "Signing identity is unavailable: $signing_identity"
    exit 1
fi

notary_profile="${AGENT_NOTARY_PROFILE:-apple-notary}"
if [ "$notarize" = true ]; then
    xcrun notarytool history --keychain-profile "$notary_profile" >/dev/null
fi

# -----------------------------------------------------------------------------
# build
# -----------------------------------------------------------------------------

export MACOSX_DEPLOYMENT_TARGET=15.0
export CARGO_TARGET_DIR="$REPO_ROOT/.build/cargo"

cargo build --locked --release

swiftc -O \
    -target "$(uname -m)-apple-macosx${MACOSX_DEPLOYMENT_TARGET}" \
    -module-cache-path "$REPO_ROOT/.build/module-cache" \
    "$REPO_ROOT/src/desktop_service/service_main.swift" \
    "$REPO_ROOT/src/desktop_service/desktop_control.swift" \
    "$REPO_ROOT/src/desktop_service/local_socket.swift" \
    -o "$REPO_ROOT/.build/desktop_service"

package_id="$(cargo pkgid --locked)"
package_version="${package_id##*#}"
package_version="${package_version##*@}"

stage_dir="$(mktemp -d "$REPO_ROOT/.build/app.XXXXXX")"
trap 'rm -rf "$stage_dir"' EXIT
app_path="$stage_dir/Agent.app"

cp -RL "$PKG_MACOS/Agent.app.template" "$app_path"
/usr/libexec/PlistBuddy -c "Add :CFBundleShortVersionString string $package_version" "$app_path/Contents/Info.plist"
/usr/libexec/PlistBuddy -c "Add :CFBundleVersion string $package_version" "$app_path/Contents/Info.plist"

# -----------------------------------------------------------------------------
# sign and optionally notarize
# -----------------------------------------------------------------------------

bundle_id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$app_path/Contents/Info.plist")"
codesign --force --options runtime --timestamp --sign "$signing_identity" \
    --identifier "$bundle_id.desktop_service" "$app_path/Contents/MacOS/desktop_service"
codesign --force --options runtime --timestamp --sign "$signing_identity" "$app_path"
codesign --verify --deep --strict "$app_path"

if [ "$notarize" = true ]; then
    archive_path="$stage_dir/Agent.zip"
    ditto -c -k --keepParent "$app_path" "$archive_path"
    xcrun notarytool submit "$archive_path" --keychain-profile "$notary_profile" --wait
    xcrun stapler staple "$app_path"
    xcrun stapler validate "$app_path"
    spctl --assess --type execute "$app_path"
fi

# Keep the previous build until compilation and signing have passed.
mkdir -p "$REPO_ROOT/dist"
rm -rf "$REPO_ROOT/dist/Agent.app"
mv "$app_path" "$REPO_ROOT/dist/Agent.app"

stderr "Built: $REPO_ROOT/dist/Agent.app"
if [ "$notarize" = false ]; then
    stderr "Developer ID signed; not notarized. Use --notarize for distribution."
fi
