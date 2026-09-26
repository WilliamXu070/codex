# rust-v0.157.1 integration

## Provenance and scope

- Integration branch: `agent/upstream-0.157.1`.
- Starting fork commit and inspected `origin/main`: `3b11682945606b791374ef9bafbf45719ca4ad79`; merging `origin/main` was already up to date.
- Exact official release tag: `rust-v0.157.1`, peeled commit `36650394c5b38c2990ccf2a3457165ca3e9d9726`.
- Workspace and local lockfile package versions: `0.157.1`.
- The live Desktop checkout, remotes, production launchers, and binary symlinks were not changed. Publication, installation, runtime identity, and launch verification belong to the release orchestrator; this is a source integration candidate.

## Saved local changes

Applied `.codex-release-context/dirty.patch` with `git apply --3way` and reviewed conflicts. The patch is based on older blobs; its intentional source changes already exist in the starting fork, often extracted into newer modules. Reapplying the old text would duplicate clipboard methods, waveform helpers, sound notification methods, and recording paths, remove newer APIs, and downgrade the workspace version to 0.144.6.

Preserved the current implementations of:

- `/sound` dispatch and popup, random completion/approval selection, immediate question sounds, and `scripts/test-codex-sound-path.sh`.
- Transcription menus, API-key input, capture/hold/release handling, RMS thresholds, live level sampling, protected waveform markers, and `william/transcribe/record-wav-live.py` and `transcribe-command`.
- Clipboard polling, paragraph/path repair, the six clipboard regression tests, and upstream's newer copy format/ownership API.
- The durable release agent, updater, polling/webhook installers, retention logic, and incident documentation. The old patch's stash-based updater is superseded by the existing isolated-clone agent; restoring it would undo the release-watcher repair.
- Saved sound-state preferences: disabled, volume 1.00.

Every intentional path in `untracked.json` already exists in the fork: clipboard repair, three ticket documents, webhook server/installer, sound regression script, and live WAV recorder. Their saved versions were compared with the current files. Kept the extracted regression tests, newer updater implementations and updated verification history. Excluded `.bazelversion 2` and all saved `*.snap.new` files. Release context remains intact.

## Semantic adaptations

- Keep upstream's owned/fullscreen transcript, raw mode, keymap inventory, history hydration, and stream rendering architecture. Restore its raw-mode configuration, dispatch, status items, and tests alongside automatic clipboard repair.
- Use the current markdown renderer and inline-visualization context for clipboard repair.
- Combine macOS accessibility/date-formatting and pasteboard dependency features in one target dependency table.
- Keep custom context RPCs alongside upstream rollout compression.
- Keep the 200,000-byte guardian action cap using upstream's complete, projected JSON representation; retain the stdin approval size check without the removed truncation wrapper.
- Route asynchronous questions to the existing attention-sound event and preserve notification coalescing.
- Normalize shifted C0 control characters without dropping modifiers so the existing Ctrl+Shift+D regression works with the upstream key matcher.

## Validation results

- `cargo check -p codex-tui --tests --offline`: passed.
- The `just test` recipe's equivalent nextest invocation (`RUST_MIN_STACK=8388608 NEXTEST_PROFILE=local cargo nextest run --no-fail-fast -p codex-tui --lib`) passed all 92 selected tests. The filter covered clipboard repair, transcription, keymap setup, the default command menu, raw output, question notifications, key normalization, and protected elements.
- Reviewed and accepted eight newly generated keymap snapshots. Updated the clipboard path regression to verify intact path spans and repaired output independently of upstream's theme-selected colors.
- `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`: passed completion, approval, and request-user-input routes.
- Executed the recorder's pure `chunk_level` function against empty/silent PCM, half-scale signed samples, full-scale negative PCM, and an incomplete trailing sample: passed RMS and peak assertions. No microphone access was attempted.
- Shell and Python syntax checks passed for the helper scripts.

## Validation constraints

- Required workspace-manifest verifier cannot start under the available Python 3.9: `ModuleNotFoundError: No module named 'tomllib'`.
- `just` execution is sandbox-blocked (`operation not permitted`), including focused tests, formatting, fix, schema generation, and Bazel lock refresh. No replacement tools were installed.
- Online `cargo shear --deny-warnings` failed resolving `index.crates.io` while requesting `futures` for `codex-git-utils`: download of `https://index.crates.io/config.json` failed with curl error 6.
- Offline shear runs and reports two upstream, comment-only unlinked files: `core/tests/suite/scenarios_read_only_mcp.rs` and `login/src/internal_identity.rs`. These warnings are not dependency regressions.
- `cargo clippy --fix --tests --allow-dirty --allow-staged -p codex-tui` could not bind its locking TCP listener: `Operation not permitted (os error 1)`.
- Non-mutating `cargo clippy --tests -p codex-tui` reaches existing `codex-context-files` dependency lint failures (36 errors: unwrap usage, a lock held across await, format arguments, redundant closures, and trivially-copyable references). That crate's source is unchanged by this integration.
- Stable `cargo fmt -- --config imports_granularity=Item` runs but warns that import granularity requires nightly. The orchestrator must rerun the canonical formatter.
- Release-script unittest discovery ran six tests successfully; two modules could not import the release agent because Python 3.9 lacks `tomllib`. Shell syntax checks passed.

The orchestrator must rerun the canonical schema/Bazel regeneration, Python 3.11+ manifest and release-script checks, formatting, full tests, and release build before publication. No runtime delivery is claimed by this integration.
