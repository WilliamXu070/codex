# Codex 0.163.0 integration handoff

Branch: `agent/upstream-0.163.0`.
Source: `2bcc5f74251f845500f18ec9fa41afa78a00533d`.
Official tag: `rust-v0.163.0`, commit `83086188ae0614bd882818ff1aa23f3194962012`.
`origin/main` was already the source commit; merging it was a no-op.

## Integration decisions

- Merged the actual official tag, including upstream daemon restart handling and
  clipboard paste targeting. Kept the fork's clipboard repair polling in the
  same event loop. The extracted parent-owned command module now also permits
  upstream `/config`.
- Ported the dirty patch's removal of the `/raw` command, public config fields,
  global shortcut and status-line item across current client-owned settings,
  history loading and keymap registration APIs. Rendering regression tests retain
  their internal raw presentation fixtures; production has no raw-mode switch.
  Existing chord tests now exercise the retained Vim toggle. Picker snapshots
  were manually reviewed and adjusted; they still require execution downstream.
- Kept current clipboard repair/copy code and its separate regression test file.
  The older dirty additions were already present, including macOS pasteboard
  dependencies, native clipboard construction and canonical markdown repair lines.
- Kept `/sound`, completion/approval selection, request-user-input notifications,
  audio helpers and the stronger sound-path regression already in the fork.
- Kept native upstream `/voice`, voice keymaps and native transcription. Excluded
  historical capture waveform/UI patches: they depend on retired custom capture.
  No `/transcribe`, `/transcribe-command`, global transcribe shortcut, Ctrl+Shift+D
  or collapsed Ctrl+D capture was restored. Archived transcription scripts remain
  inactive; the installer only removes old helper symlinks.
- Kept the newer release-agent updater, watch/webhook scripts and regression tests
  instead of reverting to the dirty patch's superseded in-place updater. Kept the
  existing external Cargo target policy and production-launcher restrictions.
- Inspected every untracked-context entry: real source, helper scripts and ticket
  documentation already existed in the fork, sometimes with newer fixes or tests.
  Preserved those versions. Excluded all `*.snap.new` and `.bazelversion 2`.
- Workspace version is `0.163.0`; refreshed workspace package versions in the
  Cargo lockfile and removed the stale lock entry for the upstream-deleted
  shell-escalation crate. No new third-party dependency was introduced.

## Validation and publication limits

- PASS: `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`.
- PASS: `git diff --check`; no unresolved merge markers or index conflicts.
- `cargo fmt -- --config imports_granularity=Item` runs successfully, but the
  installed stable formatter warns that `imports_granularity` requires nightly.
- `python3 .github/scripts/verify_cargo_workspace_manifests.py` is blocked:
  Python 3.9 raises `ModuleNotFoundError: No module named 'tomllib'`.
- `cargo shear --deny-warnings` is blocked fetching `rmcp` from
  `https://github.com/modelcontextprotocol/rust-sdk` at
  `4048ac40e958fb1a7d614617a2ae8d26666ee2fa`: GitHub DNS resolution fails;
  Cargo reports the revision unavailable in its local cache.
- `just` execution is denied by this sandbox (`operation not permitted`). This
  blocks `just fmt`, config-schema generation, Bazel lock refresh, scoped Clippy,
  and focused TUI tests (`custom_settings_commands`, `clipboard_repair`, and
  `keymap::voice_tests`). No replacement tools were installed.
- Python release-script discovery ran six tests successfully; the release-agent
  and retention modules could not import because `tomllib` is unavailable.

The orchestrator must regenerate the config schema and Bazel lock, verify
manifests, run focused TUI/keymap/snapshot tests, format/lint and build with normal
dependency access before publication. The schema deletion was applied manually.
This is a source integration handoff, not a validated runtime release: no live
checkout, production launcher, runtime symlink, remote or publication was changed.
Release context remains intact. Runtime deployment, identity/uniqueness and launch
verification remain the orchestrator's completion gates.
