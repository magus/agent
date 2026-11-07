# component-menu-items

## Context

Clarify the desktop permissions entry and give components a consistent list-based menu interface.

## Decisions

Both components expose menu_item_list returning Vec<MenuItem>. Menu construction remains free of startup
side effects. The user preferred menu_item_list over menu_items; this names the collection
without tying the function name to Vec. Name the desktop action OpenDesktopPermissions so its handler is easy to identify.

## Changes

Updated the desktop entry and documentation to Desktop permissions…. Renamed the action and changed
App to combine the component menu lists. Renamed the ChatGPT menu module to match its export.
Following the user's refinement, App starts with an empty Vec and extends it for each menu group.
Both components now expose the same module-level init and menu_item_list functions. Moved desktop
initialization, menu construction, and diagnostics into their own named files. DesktopService and
its process methods are private to the desktop component; App no longer imports the type.

## Verification

Formatting, Clippy across all targets, and diff checks passed with the final names.
