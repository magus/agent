# desktop-service-source-folder

## Context

Separate the desktop service README from its Swift source files.

## Decisions

Keep README.md at the service root and put Swift files in its src/ folder.

## Changes

Moved the three Swift files into src/desktop_service/src/ and updated the packaging script.

## Verification

Swift type-checking at the macOS 15 deployment target, shell syntax, and diff checks passed.
