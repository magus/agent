# TODO

- Use [Pierre Words for markdown rendering](docs/plans/markdown-rendering/README.md)

- Native UI follow-ups
  - Verify shared chat and an authorized remote tool call on a physical iPhone.
  - Verify extending text selection with touch handles on iPhone.
  - Full VoiceOver navigation remains deferred at Noah's request.
  - Measure and reduce long-history startup memory. Offscreen renderers are released, but every
    message is still measured during initial loading.

- Central account switching and an in-app usage display remain deferred.

- sort sidebar conversations by time of last user message sent, recent first

- allow computer tool while unlocked, oddly requires locked currently

- menu bar item to quickly see devices, active conversations, desktop service status, etc.

- display current detected computer use permissions and allow quickly opening the corresponding system settings pages to enable
  what determines when an app shows up in the system settings?
  ideally we can have the Agent entry there so the user can easily just toggle it on?
  also i was having a conversation with a model when accessibility permissions were missing and it just said computer use was unavailable
  it would be better if the model understood WHY and could tell me exactly what to do to get it enabled
  that means we might need to adjust how that error state is shown to the model so it knows

- can openai api expose web search to model or do we have to build something equivalent ourselves?

- Dock is not visible in desktop service screenshots
  figure out if we can make it visible and interact with it, that would be very useful

- allow editing the human readable names for devices we register
  defaulting to the name reported by the device is perfect, if we aren't doing that ensure we are

- add a device api + tool call for uploading files to a device

- does desktop service not work if the mac device is unlocked?
  surprising but maybe reasonable decision. might be fine to remove this limitation.
  maybe our action to get screenshot can return the screenshot along with the state of the device like the lock state or other relevant details?
