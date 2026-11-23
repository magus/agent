# TODO

- Native UI follow-ups
  - Verify shared chat and an authorized remote tool call on a physical iPhone.
  - Verify extending text selection with touch handles on iPhone.
  - Full VoiceOver navigation remains deferred at Noah's request.
  - Measure and reduce long-history startup memory. Offscreen renderers are released, but every
    message is still measured during initial loading.

- Central account switching and an in-app usage display remain deferred.

- what happens when we ask it to say view a tweet or access something like the latest stock price?
  can it use web/sonic? we need to make sure we are roughly matching capabilities available in codex/claude

- decide whether the desktop helper should stop sharing Agent's bundle ID
  it registers with LaunchServices as a UI element under `com.iamnoah.agent` because it initializes
  `NSApplication`, so tools that look up the app by bundle ID can land on the helper. Orphans are now
  prevented, so this is rare. Options: drop `NSApplication.shared` from the helper (may be what gives it a
  window-server connection while locked; needs a locked test on the mini) or move it into a nested helper
  app with its own identifier (needs Accessibility and Screen Recording re-granted)

- menu bar item to quickly see devices, active conversations, desktop service status, etc.

- computer use permissions: the model and the Mac UI now get the specific missing permission and the
  fix, and Desktop Permissions… opens each System Settings pane. Still open: what determines when Agent
  appears in those System Settings lists, so the user can simply toggle it on.

- can openai api expose web search to model or do we have to build something equivalent ourselves?

- Dock is not visible in desktop service screenshots
  figure out if we can make it visible and interact with it, that would be very useful

- allow editing the human readable names for devices we register
  defaulting to the name reported by the device is perfect, if we aren't doing that ensure we are

- add a device api + tool call for uploading files to a device
