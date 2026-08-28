# Deliver custom sound for interactive user-input prompts

## Symptom

The configured `notify` command plays the completion sound when a Codex turn
ends, but an agent `request_user_input` question is silent.

## Expected behavior

When the local Codex TUI receives the app-server
`item/tool/requestUserInput` request, it invokes the existing custom sound
script with the `request-user-input` event. Completion sounds remain unchanged.

## Diagnosis

`notify` is documented and implemented as a completed-turn hook. The TUI
already receives `ToolRequestUserInput` directly, but routes it through the
generic `PlanModePrompt` notification. That path only emits a terminal desktop
notification and never invokes the custom sound script. The external SQLite
watcher is an obsolete workaround: it expects a legacy
`response.output_item.added` wrapper that is absent from the current journal.

ChatGPT.app is a separately signed application with its own embedded Codex
runtime, so a local CLI build cannot alter that private event stream.

## Plan

1. Add a distinct TUI user-input notification type and map it to the existing
   custom sound event.
2. Route only `ToolRequestUserInput` through that type.
3. Add a unit regression test, build the local debug Codex release, activate
   its symlink, and verify the direct sound command in dry-run mode.

## Verification

- `just fmt` and `git diff --check` passed in the exact 0.148.0-alpha.22
  source worktree.
- `just test -p codex-tui request_user_input`: 88 passed.
- Fresh `codex` and `codex-tui` binaries both report `0.148.0-alpha.22` and
  the active `~/.local/bin` links match their SHA-256 hashes.
- The installed `codex-random-sound --event request-user-input` hook selects
  the immediate sound in dry-run mode.

## Status

Complete for the local terminal Codex runtime. GitHub Issues are disabled for
this repository, so this local ticket is the tracking record. ChatGPT.app is
unchanged because its signed embedded runtime is separate.
