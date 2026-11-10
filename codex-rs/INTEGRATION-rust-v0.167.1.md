# Codex 0.167.1 integration handoff

Branch: `agent/upstream-0.167.1`.
Starting fork and local `origin/main`: `2bcc5f74251f845500f18ec9fa41afa78a00533d`.
Official annotated tag object: `595a35e837a1b6752e60a5de85b89adf415b0ccb`.
Official release commit: `0a94398fa60c4fac0e7659e4213a0a36e613dd46`.

The `origin/main` merge was already up to date. Merge commit `c4a96c055`
retains both histories and the exact official release as ancestors. This imports
the complete release, including its multi-backend TUI, daemon recovery,
Markdown async questions and protocol changes, rather than only a version bump.

## Conflict resolutions

- Retained upstream daemon feature comparison and removed-feature handling,
  recovery UI and tests. Preserved the fork's agents-overview exemption from
  local profile exclusions and daemon feature reconciliation.
- Kept both clipboard polling and upstream asynchronous daemon restart in the
  new startup select loop. Each branch yields `AppRunControl::Continue`.
- Adopted upstream Markdown async question rendering and its multiline/link
  regression test and snapshot, superseding the fork's older plain-text fix.
- Kept the extracted parent-owned command allowlist module, updating its contents
  to upstream's allowlist, including Config and Title/Statusline argument handling.
- Kept both the fork's context tests and upstream critical-path tests, and both
  custom settings and upstream quick-dial test modules.
- Preserved canonical clipboard Markdown extraction alongside upstream question
  title tracking in assistant cells. Clipboard repair tests remain extracted.
- Retained the new upstream MCP discovery snapshot after its rename.
- Reconciled Cargo.lock against current-source Cargo resolution. Removed stale
  orphaned historical packages; all 176 workspace packages report 0.167.1.

## Saved changes audit

Reviewed all 50 paths in `.codex-release-context/dirty.patch` and attempted
`git apply --3way`. The patch targets 0.141.0/0.144.6 and was already ported
through earlier integrations, including `INTEGRATION-rust-v0.162.1.md`.
Current implementations preserve its intentional behavior without restoring
historical APIs or duplicating functions extracted into modules.

- External Cargo target defaults and production launcher protection remain in
  AGENTS.md and justfile. No development artifact activates production.
- `/sound` routes to the local settings popup and submits no model turn.
  Completion, approval and user-input sounds retain their helper scripts and
  repository-relative sound fallback. Async questions also use the attention route.
- Clipboard repair remains enabled in the startup loop with silent macOS
  initialization, NSPasteboard change detection, canonical unwrapped Markdown,
  paragraph/path reconstruction and six regression tests. The old inline helper
  and duplicate NSPasteboard dependency stanza were already superseded by
  `clipboard_copy/macos.rs` and merged dependency features.
- Saved raw-mode removals were superseded in earlier integrations. Preserved the
  newer raw/transcript rendering, config, status surfaces, streaming setters,
  keymaps and regression tests rather than applying the obsolete removals.
- Retired custom capture state, braille markers, events, menus and keymaps remain
  absent. No `/transcribe`, `/transcribe-command`, global transcribe action,
  Ctrl+Shift+D or collapsed Ctrl+D capture was restored. Removed a stale composer
  doc comment describing the retired waveform. Native `/voice` and native voice
  transcription remain upstream implementations.
- Preserved the current isolated release-agent updater/watcher/webhook instead
  of older live-checkout mutation and symlink replacement scripts. Kept its
  release validation, deduplication and durable handoff.
- Historical package-version edits are reconciled to 0.167.1.

All eight intentional `untracked.json` entries already have tracked successors:
clipboard repair and extracted tests; the three incident documents; webhook
server and installer; sound regression script; archived live WAV recorder.
Kept their newer versions: copying the historical files would regress the
release agent, remove user-input sound coverage or re-inline tests.
Excluded every generated `.snap.new` and accidental `.bazelversion 2`.
The transcription helper edits are retained as inactive archived source.
`william/install` registers sound helpers and retires recognized custom voice
links; it was inspected, not executed.

## Validation and delivery boundary

All Cargo invocations use `/private/tmp/codex-tui-target` and
`CARGO_INCREMENTAL=0`. No replacement tools were installed.

Passed:

- Dependency-free Cargo metadata: all 176 workspace packages report 0.167.1.
- `cargo fmt -- --config imports_granularity=Item`: exit 0; final formatting
  completed after tests, with the stable-toolchain warning noted below.
- `cargo shear --deny-warnings`: no issues found.
- Focused TUI nextest run: compiled the merged source successfully, 75/80 tests
  passed, including all six clipboard repair tests, local `/sound` routing,
  copy routing and parent-owned command coverage. Two worker tests initially
  overflowed because the first direct invocation omitted the recipe stack size.
- Follow-up nextest run with `RUST_MIN_STACK=8388608 NEXTEST_PROFILE=local`:
  all 79 tests passed. This includes both previously overflowing worker tests,
  notification/user-input/async-question sound routing coverage, and upstream
  Markdown question formatting, hyperlinks and snapshot coverage.
  Cargo dependency resolution/build succeeded using available dependencies;
  there was no uncached dependency fetch failure.
- Required `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`:
  completion, approval and user-input paths pass without playing audio.
- Five webhook tests and one watcher installation test.
- Bash/Zsh syntax checks for updater, watcher, webhook, sound and archived helpers.
- Config schema JSON parses.

Environment limitations:

- `python3 .github/scripts/verify_cargo_workspace_manifests.py` fails under
  Python 3.9.6 with `ModuleNotFoundError: No module named 'tomllib'`.
  Release-agent and retention Python test modules have the same import blocker.
- `just fmt`, `just fix -p codex-tui`, `just test -p codex-tui` and
  `just bazel-lock-update` are blocked with `operation not permitted: just`.
  Focused Rust tests use the existing recipe's Cargo nextest command directly.
  The orchestrator must rerun the canonical recipes and refresh/check the Bazel
  lockfile with its normal tool access before publication.
- Three tests from the initial focused run remain blocked by sandbox permissions:
  `tests::daemon_startup_tests::daemon_feature_compatibility_respects_required_and_optional_attachment`
  and `tests::daemon_startup_tests::daemon_startup_falls_back_only_for_implicit_endpoints`
  fail at `daemon_startup_tests.rs:128` and `:273` while binding a TCP mock
  listener, with `Operation not permitted (os error 1)`.
  `clipboard_copy::wsl::tests::stalled_input_and_stalled_exit_are_reaped_before_returning`
  fails at `clipboard_copy/wsl_tests.rs:86` because invoking `ps` to inspect the
  reaped subprocess is denied with the same error. No tests were weakened.
- Stable rustfmt accepts `cargo fmt -- --config imports_granularity=Item` but
  warns that imports granularity requires nightly.

Runtime delivery remains incomplete until the orchestrator runs its clean full
build, inventories active/discoverable copies, installs an immutable release,
proves artifact identity and verifies the real production launchers end to end.
Production launchers are `~/.local/bin/codex`, `codex-tui` and
`codex-code-mode-host`, activated through `~/.local/lib/codex/current` by the
release agent. Inventory and deployment outside this isolated clone are outside
this task's authorized scope and sandbox. No live checkout, installed binary,
helper registration, launcher symlink or process was changed. No push, PR,
remote modification or publication occurred. Release context remains intact.
