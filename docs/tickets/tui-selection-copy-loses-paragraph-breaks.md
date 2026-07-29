# Repair native Terminal copies from rich Codex history

## Symptom

In the normal rich Codex viewport, selecting assistant text with macOS Terminal
and pressing `Command-C` copies display-oriented rows. Paragraph boundaries are
flattened, wrapped cyan paths can contain inserted whitespace, and Codex's
`• ` / two-space display gutter can leak into the clipboard.

## Expected behavior

Keep native Terminal selection and the rich viewport unchanged. After
`Command-C`, repair only clipboard text that uniquely matches a finalized Codex
assistant response:

- A contiguous cyan pathname, URL, command, or identifier remains one token.
- Normal prose uses spaces for visual wraps and preserves logical newlines.
- Codex's display-only bullet and two-space gutter are removed.
- Ambiguous or unrelated clipboard contents remain untouched.
- No mouse capture, copy-on-release behavior, or transcript notification is
  added.

## Diagnosis

Terminal.app consumes `Command-C`; crossterm does not receive that shortcut.
Listening for the key inside the TUI is therefore not portable. The system
clipboard can instead be polled for text changes while Codex is running.

Finalized assistant responses are retained as source-backed
`AgentMarkdownCell`s. Re-rendering one without a viewport width produces
canonical plain visible text with logical paragraph/list/code boundaries and
unwrapped cyan spans. A whitespace-insensitive, unique match from the native
clipboard back to that canonical text recovers the selected source range
without guessing from flattened text.

The previous experimental approach captured all mouse events, copied on mouse
release, and inserted `Copied selection to clipboard` history rows. It also
consumed wheel events and only modeled visible history. That implementation has
been removed.

## Plan

1. Add a macOS pasteboard text-change monitor without mouse or keyboard capture.
2. Build canonical repair documents from finalized `AgentMarkdownCell`s.
3. Match non-whitespace characters uniquely and rewrite only when the canonical
   selection differs.
4. Add exact tests for multi-paragraph prose, wrapped cyan paths, mixed text,
   display gutters, ambiguity, and unrelated clipboard text.
5. Build, install, and verify the clipboard transition through the normal rich
   TUI.

## Verification

- The original clipboard-repair implementation passed its six focused tests,
  config coverage, affected TUI snapshots, formatting, schema validation, and
  strict TUI linting before this release integration.
- The `0.146.0` integration passes `cargo shear --deny-warnings` and the sound
  path regression. The sandbox cannot complete the TUI build because the
  `rusty_v8` archive is not cached and network access is disabled; the release
  orchestrator reruns the full build and tests with dependency access.

## Status

Implementation complete. Release `0.146.0` activation remains pending the
orchestrator's full validation.
