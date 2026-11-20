# App icon directions

## Context

The user requested Agent icon explorations informed by the app's goals and Roomy's installed icon.
After reviewing twelve initial concepts and four blends, the user selected the porcelain hub with
eyes on charcoal. The user then requested a genuine SVG under 1 MB, matching that render, with
transparent exterior margins and a subtle outer shadow. Visual approval is required before acceptance
or installation.

## Decisions

- November 19: the user requested a smaller optimized sibling SVG with no visual quality loss.
  Preserve the existing SVG byte-for-byte and compare rendered outputs before delivery.
- Preserve the selected hub's silhouette, three satellite domes, eyes, porcelain, and charcoal tile.
  The earlier pointer recommendation was superseded by the user's selection.
- Fit smooth cubic paths to the render's lighting contours. Separate porcelain color corrections,
  charcoal texture, tile clipping, and the outer shadow. The SVG contains no embedded raster data.
- Recreate fine diagonal charcoal grain with SVG turbulence and a rotated pattern. Do not trace
  every grain into geometry; the initial dense trace exceeded the size budget and was replaced.
- Keep the SVG independent of app packaging until the user approves the vector result.

## Changes

- Added `assets/agent-icon.svg`, 832,288 bytes, containing 216 paths and native SVG filters.
- Generated twelve initial concepts and four blends concurrently with the built-in image tool.
  Explorations and their prompts remain in `.scratch/2026-11-11-02-20-icon-directions/`.
- Selected reference: `blend-02-porcelain-hub-face.png` in that folder. The original is
  `/Users/noah/.codex/generated_images/01a1c671-d76d-7203-ba3a-56be0b8ea785/exec-8a0d105a-9b15-4e07-988a-a21932e26cb0.png`.
- Vector fitting code and the local review page are in the exploration folder's `vector/` directory.
  Browser-rendered PNGs, side-by-side comparison, size comparison, background comparison, and
  `verification.json` are under `vector/previews/`.
- Smoothed the tile outline, removed a warm edge fringe, corrected texture brightness, and restored
  cooler porcelain rim reflections after direct visual comparisons.

## Verification

- Inspected Roomy's installed icon and the Agent README, central-service plan, and chat-UI plan.
- Rendered the SVG in installed Chrome through the built-in browser tool. Inspected the full-size
  output and 32, 64, 128, and 256 pixel comparisons. No Playwright installation or launch was used.
- Parsed the final SVG and verified its size is below 1,000,000 bytes. No image, feImage, script,
  foreignObject, data URI, or external asset reference is present. All internal references resolve.
- Verified each exported size, RGBA output, transparent corners, and a nonzero soft outer shadow.
  The production SVG and browser-tested SVG are byte-identical.
- SHA-256: `a31b2b6c15ec569fb3e32487fab7b57b5e3e9a393f57b30710a8f356b1caeafd`.
- Mean absolute RGB difference within the opaque interior is 2.470 on a 0–255 scale. This is a
  supporting measurement, not proof of visual acceptance. The recreated grain is not pixel-identical,
  and tiny porcelain reflections are slightly smoother. The exterior background was intentionally
  replaced with transparency and a rebuilt shadow.
- No app code or packaging changed. The SVG has not been installed or accepted by the user.

## Follow-ups

- Await the user's visual approval of the SVG. The goal is blocked on that explicit approval gate
  after three consecutive goal turns reached the same pending requirement. No installation occurred.

### November 19: lossless SVG optimization

- Preserved `assets/agent-icon.svg` byte-for-byte (832,288 bytes; SHA-256
  `a31b2b6c15ec569fb3e32487fab7b57b5e3e9a393f57b30710a8f356b1caeafd`).
- Added `assets/agent-icon-optimized.svg`: 731,128 bytes, 101,160 bytes (12.15%) smaller.
- Removed redundant path commands, numeric formatting, and whitespace. Preserved all coordinates,
  layers, colors, filters, texture, and accessibility text. No embedded raster images.
- Rejected SVGO candidates because browser comparisons detected pixel differences. The delivered
  version uses syntax-only cleanup, with no geometry conversions or coordinate rounding.
- Compared fresh browser canvas renders at 32, 64, 128, 256, 1254, and 2508 pixels. Every RGBA channel
  matched exactly at all six sizes. Inspected the side-by-side browser preview.
- Reproduction script, PNG renders, and pixel report:
  `.scratch/2026-11-19-15-00-icon-optimization/`.
- Original visual approval remains pending. No installed app assets were changed.

### November 19: compact reconstruction

- The user rejected the 731 KB result and requested roughly 20 KB or less.
- Rebuild the porcelain icon with a few paths, gradients, and SVG filters. Preserve both existing
  versions. This replaces traced shading with an editable compact illustration; exact pixel equality
  is no longer the validation target. Compare the appearance and deliver a preview for approval.

- Added `assets/agent-icon-compact.svg`, a 7,573-byte reconstruction using reusable dome shapes,
  gradients, highlights, shadows, and procedural texture. The previous SVG files remain unchanged.
- Inspected the browser rendering beside the original and softened the neck and arm shading.
  The compact version retains the design but has simpler material shading and tile depth. It is not
  pixel-identical. Browser PNG preview exported; XML parses and no embedded image elements exist.
- The compact file passes the 20 KB size limit. Visual approval remains with the user.

### November 19: compact edge and lighting refinement

- The user identified misaligned edges and lighting lines and supplied Roomy's SVG as a reference.
- Roomy uses shared silhouette paths for clipped bevels, layered edge shading, and repeating grain.
  Apply those techniques to the charcoal tile and porcelain silhouette. Remove independently drawn
  edge stripes that do not follow the actual outline. Keep the compact SVG below 20 KB.

- Refined the compact SVG using Roomy's shared-outline bevel technique and a reduced version of its
  repeating grain pattern. Replaced the hard tile border with softly layered edge shading.
- Clipped all porcelain shading to one silhouette. Removed detached highlight strokes, softened
  dome joins, deepened spherical shading, and aligned subtle dome reflections with their geometry.
- Compared the revision against the large SVG in the browser, including full-resolution PNG review.
  It is closer in edge alignment and material continuity but remains a compact reconstruction,
  not an exact match to the traced render. Visual approval remains pending.
- Verified XML parsing, internal reference resolution, absence of embedded images, and size under
  20 KB. Earlier large SVGs are unchanged. The prior compact version is retained in scratch.

### November 19: packaging audit and material refinement

- User approved checking packaging and addressing the six remaining visual differences.
- Both `packaging/macos/app.sh` and `packaging/apple/build.sh` assemble app bundles directly.
  Neither compiles an icon asset catalog or installs an icon. The Mac plist has no icon key and sets
  macOS 15 as its minimum version. System masking cannot be assumed for all supported builds.
- Keep the standalone rounded SVG until icon packaging is implemented. An Icon Composer source
  for newer systems and a rendered legacy icon need separate validation. No installed app changes.
- Refine the central ridge, connector reflections, highlight softness, eye recesses, directional
  texture, and contact shadows while retaining the 20 KB budget.

- Updated compact SVG to 10,953 bytes. Removed the central circular stroke, softened highlights and
  dome transitions, added curved connector reflections, widened the eye socket shading, replaced
  speckles with subtle directional texture, and strengthened contact shadows.
- Checked browser renders beside the original and inspected the full-resolution export. The compact
  version still has simpler spherical shading than the large traced SVG. This is a review candidate,
  not a claim of identical appearance or completed visual approval.
- XML and internal references validate, no embedded images exist, and the file remains below 20 KB.

### November 20: right dome join

- User spotted a visible edge where the lower-right dome meets its arm. Fade the dome's rim reflection
  at that connection instead of allowing a circular highlight to cross the continuous porcelain join.
- Applied a soft mask only to the lower-right dome's rim reflection. Browser preview confirms the
  line fades into the arm. SVG parses and remains under 20 KB. Exported a fresh review PNG.

### November 20: approved asset cleanup

- User accepted the current icon as good enough and authorized removing intermediate SVGs and
  comparison previews. Renamed the final 11,732-byte SVG to `assets/agent-icon.svg` without changing
  its contents. Removed the obsolete traced and optimized SVG versions.
- Preserved the selected generated reference render as `assets/agent-icon-render.png`.
  Verified that the copy matches the source PNG byte-for-byte. The generated source remains intact.
- Removed the optimization scratch directory and earlier vector comparison previews and SVG copy.
  Historical scratch links above are no longer available. The final artwork and reference are in
  the repository's assets directory; no installed app or packaging changes were made.
- Verified final SVG XML parsing and matching SHA-256 before and after the rename.
- Follow-up: generate and wire platform icon exports into packaging when installing the app icon.
