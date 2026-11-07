# Agent core

Next phase, after desktop-service cleanup and packaging. Not implemented yet.

## Goal

Run one user task through a Rust agent loop in Agent.app, using our existing desktop service on a
locked Mac mini. Keep the GPUI interface small and show each action with its resulting screenshot.

## ChatGPT login

Include a ChatGPT login flow and use eligible ChatGPT Pro subscription quota. Do not require an
OpenAI API key or silently switch to API billing. Show the signed-in account, available usage status,
and a clear recovery path for expired login or exhausted quota. Keep credentials out of task logs.

First prove login and one model request with the intended account. Confirm the request uses the
subscription and supports screenshot input and tool calls before building the task loop.

Evaluate [Sign in with ChatGPT](https://developers.openai.com/siwc) first. Its documentation includes
ChatGPT plan usage, client registration, model requests, and session renewal. The
[Codex app-server auth interface](https://learn.chatgpt.com/docs/app-server#auth-endpoints) also
provides managed login and rate-limit status. Choose the smallest supported integration after
checking its registration requirements and fit for our local app. This is not yet a decision to
embed Codex or to build directly against an undocumented endpoint.

## First proof

1. Sign in from Agent and confirm one screenshot-based model request uses subscription access.
2. Accept one task and capture the locked desktop through the Swift service.
3. Let the model choose a mouse, keyboard, drag, or bulk-text action using vision only.
4. Execute through the service and show the returned screenshot alongside the action.
5. Continue until the task finishes, the user stops it, or a concrete failure needs attention.

Use Calculator for the first complete task. Then verify dragging and a long text paste in another
app. The loop runs on the mini and must not depend on a connected MacBook or an unlocked desktop.

## Boundaries

- Keep the Swift service responsible for capture and input. Keep task state and model interaction
  in Rust. Use GPUI for login, task entry, progress, and stopping a task.
- No accessibility-tree queries or app-specific controllers.
- Retain action results and screenshots locally. Do not replay input after an ambiguous failure.
- Require a fresh capture after a service failure. Surface permission and login problems clearly.
- Account for the existing locked-session menu-bar limitation. Do not claim universal app support.
- Leave scheduling, messaging integrations, and multiple concurrent tasks for later.
