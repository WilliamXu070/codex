# Codex 0.155.1 integration handoff

Branch: `agent/upstream-0.155.1`.
Starting fork and supplied `origin/main`: `3b11682945606b791374ef9bafbf45719ca4ad79`.
Official `rust-v0.155.1` commit: `be2951ea34f0d295ed0becf97079f92fa5f6950e`.

`git merge origin/main` reported already up to date. The release is merged as
an actual second parent; the workspace version is 0.155.1.

## Saved source reconciliation

All intentional files in the saved dirty patch and untracked manifest were
already represented in the starting fork. `git apply --3way` was used and
each conflicting region was inspected. Existing implementations were retained
where the saved patch predates subsequent integrations and refactors:

- Keep current 0.155.1 manifests and lockfile, not the saved 0.144.6 version.
- Preserve upstream local settings, thread metadata, keymap actions, export,
  status surfaces, session startup, and Guardian action formatting APIs.
- Keep the fork's extracted composer helpers, clipboard repair tests, and
  startup-loop clipboard polling; do not duplicate the old inline definitions.
- Keep the newer durable release-agent updater/webhook scripts and historical
  ticket updates. Do not reintroduce the obsolete stash/build/symlink updater.
- Keep `/sound` routing and configuration, random completion/approval helper
  behavior, and question sound notification routing.
- Keep transcription hold/release capture, RMS sidecar and silence threshold,
  waveform rendering, provider settings, and live recording helper.
- Preserve automatic clipboard repair and the current copy picker. The fork
  intentionally removed `/raw`; remove merge-reintroduced references to that
  missing API and trigger resize reflow directly in the existing transcript
  restoration regression test. Keep its existing snapshot coverage.
- Keep both spoken assistant rendering and clipboard canonical-line rendering.
- Follow upstream Guardian's String-returning formatter and metadata separation,
  including the corresponding MCP elicitation assertion.
- Reconcile keymap snapshot counts: 145 actions, 21 common actions.

Generated `*.snap.new` files and accidental `.bazelversion 2` were excluded.
Release context remains intact and locally ignored.

## Validation

- PASS: sound-path shell regression (completion selection, approval selection,
  and interactive question routing), with `CODEX_ROOT` set to this clone.
- PASS: `cargo fmt -- --config imports_granularity=Item` (exit 0).
  Stable rustfmt warns that `imports_granularity=Item` requires nightly, so
  import-granularity enforcement still needs the orchestrator's formatter.
- PASS: `git diff --check`, conflict-marker/intentional-file inspection,
  workspace version inspection, and ancestry checks for fork and release.
- BLOCKED: `python3 .github/scripts/verify_cargo_workspace_manifests.py`:
  system Python is 3.9.6; `ModuleNotFoundError: No module named 'tomllib'`.
- BLOCKED: `cargo shear --deny-warnings` and focused TUI nextest execution:
  `failed to get futures as a dependency of package codex-git-utils v0.155.1`;
  download of `https://index.crates.io/config.json` failed with
  `[6] Couldn't resolve host name (Could not resolve host: index.crates.io)`.
- BLOCKED: `just test -p codex-tui clipboard_repair`, `just fmt`,
  `just fix -p codex-tui`, `just write-config-schema`, and
  `just bazel-lock-update`: `zsh: operation not permitted: just`.
  No tools were installed or replaced.

Because just could not execute, the existing test recipe was also attempted
via `cargo nextest run --no-fail-fast -p codex-tui` with filter:
`test(clipboard_repair) | test(slash_copy) | test(transcribe) | test(sound) | test(request_user_input) | test(transcript_viewer_close)`.
It failed during Cargo metadata/dependency resolution before tests executed.
Cargo commands used `CARGO_TARGET_DIR=/private/tmp/codex-tui-target` and
`CARGO_INCREMENTAL=0`; nextest also used the recipe's stack/profile settings.

## Publication boundary

This is a committed source integration, not a validated or deployed release.
The orchestrator must repeat manifest validation, formatting, shear, schema and
Bazel lock checks, focused tests/snapshots, and the full build with dependency
access before publication. Runtime freshness/identity and exact launcher
verification remain the orchestrator's responsibility after immutable release
installation. No live checkout, installed launcher, symlink, remotes, or
publication state was changed. Nothing was pushed and no PR was opened.

## Saved-file inventory

Every path below is tracked and retained (some already superseded as described
above). No intentional untracked source or helper was missing.

Tracked dirty patch (50 files):

- `AGENTS.md`
- `codex-rs/Cargo.lock`
- `codex-rs/Cargo.toml`
- `codex-rs/config/src/tui_keymap.rs`
- `codex-rs/config/src/types.rs`
- `codex-rs/core/config.schema.json`
- `codex-rs/core/src/config/config_tests.rs`
- `codex-rs/core/src/config/mod.rs`
- `codex-rs/thread-manager-sample/src/main.rs`
- `codex-rs/tui/Cargo.toml`
- `codex-rs/tui/src/app.rs`
- `codex-rs/tui/src/app/event_dispatch.rs`
- `codex-rs/tui/src/app/input.rs`
- `codex-rs/tui/src/app/resize_reflow.rs`
- `codex-rs/tui/src/app_event.rs`
- `codex-rs/tui/src/bottom_pane/chat_composer.rs`
- `codex-rs/tui/src/bottom_pane/mod.rs`
- `codex-rs/tui/src/bottom_pane/slash_commands.rs`
- `codex-rs/tui/src/bottom_pane/snapshots/codex_tui__bottom_pane__command_popup__tests__command_popup_default_items.snap`
- `codex-rs/tui/src/bottom_pane/status_line_setup.rs`
- `codex-rs/tui/src/bottom_pane/status_line_style.rs`
- `codex-rs/tui/src/bottom_pane/status_surface_preview.rs`
- `codex-rs/tui/src/chatwidget.rs`
- `codex-rs/tui/src/chatwidget/constructor.rs`
- `codex-rs/tui/src/chatwidget/interaction.rs`
- `codex-rs/tui/src/chatwidget/notifications.rs`
- `codex-rs/tui/src/chatwidget/slash_dispatch.rs`
- `codex-rs/tui/src/chatwidget/status_surfaces.rs`
- `codex-rs/tui/src/chatwidget/tests/plan_mode.rs`
- `codex-rs/tui/src/chatwidget/tests/slash_commands.rs`
- `codex-rs/tui/src/chatwidget/tests/status_and_layout.rs`
- `codex-rs/tui/src/chatwidget/tool_requests.rs`
- `codex-rs/tui/src/clipboard_copy.rs`
- `codex-rs/tui/src/history_cell/messages.rs`
- `codex-rs/tui/src/keymap.rs`
- `codex-rs/tui/src/keymap_setup/actions.rs`
- `codex-rs/tui/src/lib.rs`
- `codex-rs/tui/src/slash_command.rs`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_all_tab_search.snap`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_custom.snap`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_fast_mode_enabled.snap`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_first_actions.snap`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_narrow.snap`
- `codex-rs/tui/src/snapshots/codex_tui__keymap_setup__tests__keymap_picker_wide.snap`
- `codex-rs/tui/src/streaming/controller.rs`
- `justfile`
- `scripts/install-codex-release-watch.sh`
- `scripts/update-codex-local.sh`
- `william/audio/random-sound`
- `william/transcribe/transcribe-command`

Intentional untracked manifest (8 files):

- `codex-rs/tui/src/clipboard_repair.rs`
- `docs/tickets/2026-08-01-unbounded-local-build-cache-retention.md`
- `docs/tickets/2026-08-26-request-user-input-sound.md`
- `docs/tickets/tui-selection-copy-loses-paragraph-breaks.md`
- `scripts/codex-release-webhook-server.py`
- `scripts/install-codex-release-webhook.sh`
- `scripts/test-codex-sound-path.sh`
- `william/transcribe/record-wav-live.py`
