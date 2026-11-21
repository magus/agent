# App icon packaging

## Context

Use the approved Agent icon in the Mac and iPhone apps. Let Apple apply icon masks rather than
baking rounded corners into the app artwork. Preserve the original SVG and reference image.

## Decisions

- The square SVG is the app-icon source of truth. Regenerate its checked-in PNG during every
  build and provide a standalone export command. Keep build orchestration in Python.

- Prepare a square icon asset from the approved SVG, preserving its foreground artwork.
- Use Apple's icon tooling for platform masks and legacy exports where supported.

## Changes

- Added `packaging/apple/render_icon.py` and call it before icon compilation on every platform.
  It installs pinned resvg 0.45.1 into the project build cache and exports the square SVG. The PNG
  remains checked in for opening Icon Composer directly, but is generated rather than edited.
  Documented the source, output, and standalone export command in the packaging README.

- Added `assets/Agent.icon` with square artwork, without the original tile mask or outer shadow.
  Preserved the original SVG and reference PNG. Exported the square SVG with resvg 2.6.2 because
  direct SVG import into Icon Composer lost filter softness. The square SVG stays beside its PNG
  inside the icon document for editing.
- Added Python icon compilation to the Apple build. Xcode generates `Assets.car`, platform icon
  images, and the legacy Mac `.icns`; generated plist keys are merged before signing.
- User accepted Icon Composer's first-use license directly. Previewed the system-masked icon.

## Verification

- The automatic export and Simulator build pass. Repeated exports are byte-identical. The native
  CLI export differs in bytes from the earlier Node wrapper export; visual inspection confirms
  the artwork is retained. A proposed pixel comparison could not run because Pillow is absent.
  All three existing Python signing tests pass.

- Mac and Simulator builds pass with compiled icon resources and generated icon metadata.
- The Mac build and published `dist/Agent.app` pass strict signature verification.
- Finder Get Info displays the icon. Installed the Simulator build and verified the Agent icon on
  the iPhone Home Screen. Physical iPhone and older macOS visual checks were not run.
- Apple's compiler generated the legacy Mac `.icns` automatically. Original artwork is unchanged.
