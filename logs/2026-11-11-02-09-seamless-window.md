# Seamless chat window

## Context

Match Roomy's window treatment so Agent's chat has no separate native title bar.

## Decisions

Use Roomy's transparent title bar and traffic-light position. Keep the existing chat controls.

## Changes

Place the chat toolbar beside the native window controls on the same background.

## Verification

Formatting and `git diff --check` passed. The release app built and signed successfully.
Reopened the local app and visually confirmed the continuous background and toolbar alignment.
The existing service connection and conversation loaded. This local build is not notarized.
The sandbox could not see the signing identity; the approved build outside it succeeded.
