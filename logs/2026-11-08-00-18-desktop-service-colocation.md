# desktop-service-colocation

## Context

Keep the Rust desktop-service wrapper and Swift helper together as one component.

## Decisions

Place native source in app/desktop_service/swift/ and keep the component README alongside Rust.
Compilation, executable destinations, and signing remain unchanged.

## Changes

Moved the Swift files and README, updated packaging source paths, and corrected documentation links.
Removed the empty src/desktop_service/ directory.

## Verification

Swift type-checking at the macOS 15 target, shell syntax, diff checks, and local README link
validation passed. No app rebuild or restart was needed for this source-only move.
