# login-and-loop-scope

## Context

Clarify background token refresh and defer screenshot testing to agent-loop development.

## Decisions

Refresh must use saved credentials without browser interaction. Screenshot input belongs with
computer-use tools and the task loop, not a separate prerequisite.

## Changes

Updated docs/plans/agent-core/README.md with the revised sequence and refresh expectations.

## Verification

Read the existing refresh path. It sends the saved refresh token directly to the token endpoint
and saves the renewed credentials. It does not open a browser. Live refresh remains untested.
