# Markdown rendering with Pierre Words

## Direction

Use [Pierre Words](https://diffs.com/words), `@pierre/words`, for Agent's markdown rendering.
Noah prefers Pierre's products and explicitly requested this direction on November 18, 2026.
This is planned work; Words is not integrated yet.

## Fit with the native app

The [native UI plan](../native-ui/README.md) remains the app architecture. Current message rendering
uses `src/apple/Chat/MarkdownContent.swift` and `MarkdownBlock.swift` with Swift attributed strings.

[Pierre's documentation](https://diffs.com/docs) describes Words as beta. It provides HTML and DOM
renderers without requiring React, with WebAssembly parsing and support for streamed markdown.
Syntax highlighting is optional. This is a web integration, not a drop-in SwiftUI component.

Start with a small, locally bundled Words content view hosted in `WKWebView`. Treat this as the
integration approach to prove, not a completed architecture decision. Keep conversation state,
navigation, the composer, and tool actions in the native app.

## Implementation checklist

- [ ] Confirm the current package API, license, version, and JavaScript/WebAssembly support on the
  app's supported Mac and iPhone versions. Pin the selected version.
- [ ] Build a bundled, offline message-rendering proof on both platforms using the non-React DOM
  renderer. Avoid loading runtime code from a CDN.
- [ ] Resolve content height, native conversation scrolling, selection across paragraphs, copying,
  link opening, focus, and view reuse before choosing the final hosting layout.
- [ ] Preserve incremental rendering and scroll position as messages update. Verify incomplete
  markdown during updates and exact final rendering when a response finishes.
- [ ] Verify paragraphs, headings, lists, quotes, tables, links, inline code, and fenced code.
  Add Pierre's optional highlighting integration where needed.
- [ ] Define image loading and link policies for model-generated content. Use the documented HTML
  or DOM renderer checks and audit any custom rendering hooks.
- [ ] Verify light/dark appearance, Dynamic Type, VoiceOver, keyboard selection, Reduce Motion,
  long conversations, and large code blocks on Mac and iPhone.
- [ ] Replace the existing Swift markdown renderer once the proof passes. Preserve message source
  text and copying behavior, then remove obsolete rendering code and update its tests.

## Completion

Both native clients use Words for message markdown with the checks above verified through actual
interaction. Record screenshots, performance observations, and any limitations in the delivery log.
If a native integration limit prevents adoption, record the evidence and resolve that specific limit
with Noah before choosing another renderer.
