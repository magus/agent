# rust-module-paths

## Context

Move shared assets to the repository root and remove Rust module path overrides.

## Decisions

- Use Rust's snake_case module filenames. Keep the existing type names.
- Keep shared assets in assets/ and embed the logo relative to Cargo's manifest directory.

## Changes

- Renamed the six PascalCase Rust files and removed their path attributes.
- Moved openai.svg to assets/ and removed the empty module-local assets directory.

## Verification

- Formatting, Clippy across all targets, and diff checks passed. Compilation resolved the
  renamed modules and embedded the logo from its new location. No path attributes remain.
