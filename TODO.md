# TODO

- can openai api expose web search to model or do we have to build something equivalent ourselves?

- Dock is not visible in desktop service screenshots
  figure out if we can make it visible and interact with it, that would be very useful

- allow editing the human readable names for devices we register
  defaulting to the name reported by the device is perfect, if we aren't doing that ensure we are

- add a device api + tool call for uploading files to a device

- does desktop service not work if the mac device is unlocked?
  surprising but maybe reasonable decision. might be fine to remove this limitation.
  maybe our action to get screenshot can return the screenshot along with the state of the device like the lock state or other relevant details?
