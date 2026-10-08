# Codex 0.162.0 integration handoff

Branch: `agent/upstream-0.162.0`.
Source fork and local `origin/main`: `3b11682945606b791374ef9bafbf45719ca4ad79`.
Official annotated tag: `rust-v0.162.0`, tag object
`1f3f93473394b620b35580859b7e6864f7a9f948`, release commit
`c1382380de69521303b416720a52f42d51af6248`.

`origin/main` was already an ancestor, so its merge was a no-op. The official
release was merged with both parents retained. No remote references were changed.

## Saved local changes

Applied `.codex-release-context/dirty.patch` with three-way merging and compared
its conflicts with the merged source. The intentional sound, dictation, RMS,
braille waveform, clipboard repair, keymap, title, build-cache, and updater work
was already present in the fork, including later refactoring and fixes. Kept those
implementations rather than duplicating their earlier inline forms. Reconciled the
old version edits to 0.162.0; aligned every workspace package in Cargo.lock with
Cargo's dependency-free workspace metadata.

All eight intentional entries in `untracked.json` already had tracked successors:
clipboard repair (with extracted tests), three incident documents, the webhook
server and installer, sound-path regression script, and live WAV recorder. Compared
each saved file with its tracked successor. Kept the newer durable release agent,
signed-webhook validation and duplicate-delivery handling. Did not revive the old
updater's live-checkout stash/build/symlink workflow. Excluded every `.snap.new`
and `.bazelversion 2`. Release context remains intact and ignored.

Upstream fullscreen transcript, semantic copying, raw output, keymap inventory,
MCP and Guardian architectures are retained. The old dirty patch's removal of raw
output is superseded by the current release's transcript architecture; both rich
clipboard repair and upstream raw mode remain usable. Guardian now bounds whole
review requests instead of using the fork's obsolete formatter return type.

Adaptations include merging macOS dependency features, retaining silent native
clipboard initialization in a separate module, using the current Markdown renderer,
using upstream selection-aware element insertion for protected transcription
markers, removing obsolete slash-command references, and routing async questions
to the existing attention sound. Updated keymap snapshots for transcription and
retained upstream raw-mode tests. Removed the orphaned command-popup snapshot
whose test upstream deleted. Added a local popup routing regression for `/sound`,
`/transcribe`, and `/transcribe-command`.

## Validation

Passed:

- `cargo metadata --no-deps --format-version 1`: all 166 workspace packages are 0.162.0.
- `cargo fmt -- --config imports_granularity=Item` from `codex-rs`: successful;
  installed stable rustfmt warns that `imports_granularity` requires nightly.
- Required `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`:
  completion, approval and user-question sound routes pass without playing audio.
- Two isolated rustc tests compile the actual waveform module and verify silence,
  RMS-to-braille mapping, amplitude clamping and width capping.
- Five release-webhook unit tests and one watcher-install regression pass.
- Bash/Zsh syntax checks for release, sound and transcription helpers pass.

Blocked and required before publication:

- `python3 .github/scripts/verify_cargo_workspace_manifests.py` and release-retention
  tests: available Python is 3.9; `ModuleNotFoundError: No module named 'tomllib'`.
- `cargo shear --deny-warnings` and focused TUI nextest tests cannot fetch
  `https://github.com/openai-oss-forks/crossterm?rev=ed1cdab335221515706178d68495bba2aed1924f`.
  Cargo reports `revision ed1cdab335221515706178d68495bba2aed1924f not found`, caused
  by `failed to resolve address for github.com: nodename nor servname provided,
  or not known; class=Net (12)`. Compilation never started.
- `just` is inaccessible (`operation not permitted`), so `just fmt`, scoped fix,
  config-schema regeneration and Bazel lock regeneration could not run. The
  focused test attempt used the existing nextest command from the just recipe,
  with `RUST_MIN_STACK=8388608`, `NEXTEST_PROFILE=local`, and filter
  `test(sound) | test(transcribe) | test(clipboard)`.
- Re-run those checks, including the new custom-settings test, all affected TUI
  tests, snapshot acceptance, app-server/protocol tests, schema generation and
  Bazel lock regeneration with normal dependencies and Python 3.11+. The config
  schema was reconciled with the upstream schema plus the custom transcribe key.
  MODULE.bazel.lock has not been regenerated locally.

All Cargo commands used `/private/tmp/codex-tui-target` and disabled incremental
state. No replacement tools were installed. The full suite/build and runtime
freshness verification remain the orchestrator's publication gate. This is a
source integration handoff, not a validated runnable release. No production
launcher, live binary, live checkout, or installed helper was changed; there was
no push, PR, deployment or activation.
