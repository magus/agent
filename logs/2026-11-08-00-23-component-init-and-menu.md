# component-init-and-menu

## Context

Separate initialization side effects from menu construction. The old register functions did both.

## Decisions

Use init for component startup and menu_item for side-effect-free menu entries.
Keep mod.rs files limited to declarations and exports.

## Changes

Updated App to initialize both components before constructing its menu. Split ChatGPT registration
into init.rs and menu_item.rs. DesktopService now exposes separate init and menu_item methods.

## Verification

Formatting, Clippy across all targets, and diff checks passed. An initial visibility error in the
menu re-export was corrected. Confirmed menu_item functions only construct entries and no register
calls remain in app code. No runtime behavior change was intended; the app was not restarted.
