# Codex rust-v0.167.0 integration handoff

Branch: `agent/upstream-0.167.0`.

The integration merges the exact official `rust-v0.167.0` tag into personal source
`2bcc5f74251f845500f18ec9fa41afa78a00533d`. `origin/main` already equals that
personal source commit, so its required merge is a no-op. Both histories are
ancestors of the resulting branch. Workspace and local lockfile package versions
are 0.167.0.

## Source reconciliation

The supplied dirty patch was applied with three-way resolution after excluding a
snapshot path removed by upstream. Most of its edits predate the starting fork:
they were already ported, improved, or deliberately retired in that history.
Retained the current implementations rather than reverting them to historical
0.144.6 source. In particular:

- Preserve `/sound`, sound popups/settings, completion and approval hooks,
  immediate question sounds, random sound scripts, and the sound-path regression.
- Preserve clipboard copying and pasteboard repair, including extracted tests,
  macOS clipboard support, logical paragraph boundaries and history polling.
  The event loop retains both clipboard polling and upstream daemon restart.
- Keep upstream native `/voice`, voice capture/transcription, markdown question
  rendering and question metadata. Preserve custom settings test registration
  alongside upstream quick-dial tests.
- Adapt the extracted parent-command routing module to upstream's Config command
  and argument-bearing Title/Statusline commands.
- Keep the newer release-agent updater and launchd/webhook scripts. The old dirty
  updater would reinstate direct mutable production builds and duplicate option
  parsing; its intended update functionality is already superseded.
- Preserve source and tickets listed in untracked.json. They already exist in
  the starting fork: clipboard tests were extracted; scripts and tickets include
  later fixes; archived record-wav-live.py differs only in Python formatting.
- Exclude generated `.snap.new` files and `.bazelversion 2`. Keep custom capture
  helper source inactive. No custom transcription command, global capture action,
  Ctrl+Shift+D/normalized Ctrl+D capture binding, or helper registration restored.

The original release context is preserved unchanged.

## Validation

Passed:

- `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`:
  `codex sound path regression passed` (completion, approval, interactive question).
- `cargo fmt -- --config imports_granularity=Item` from codex-rs, exit 0.
  Stable rustfmt warns that imports_granularity requires nightly.
- `git diff --check` and both required ancestry checks.
- Six release watcher/webhook Python tests passed in unittest discovery.

Blocked, requiring orchestrator validation before publication:

- `python3 .github/scripts/verify_cargo_workspace_manifests.py`:
  Python 3.9 raises `ModuleNotFoundError: No module named 'tomllib'`.
  Release-agent and retention Python test modules hit the same import failure.
- `cargo shear --deny-warnings`: Cargo cannot refresh the read-only cached v8 git
  checkout for `https://github.com/seanh-oai/rusty_v8.git`, revision
  `350c5a859a9b3c84ab8701b3ad51e3b7699b30c0`:
  `failed to remove directory /Users/williamxu/.cargo/git/checkouts/rusty_v8-1c356b58a53d1d98/350c5a8/v8`,
  `Operation not permitted (os error 1)`.
- `just test -p codex-tui clipboard_repair`, `just fmt`, and
  `just bazel-lock-update`: `zsh: operation not permitted: just`.
  Focused Rust sound/routing/clipboard and daemon integration tests remain unrun.
- Candidate `cargo build -p codex-cli -p codex-tui --offline`, with
  CARGO_TARGET_DIR=/private/tmp/codex-tui-target and CARGO_INCREMENTAL=0,
  fails at the same cached rusty_v8 permission error before compilation.

No replacement tools were installed. No full test suite was run.

## Runtime delivery boundary

This is a source-integration handoff, not a completed runtime release. Candidate
build output is `/private/tmp/codex-tui-target/debug/{codex,codex-tui}`. Publication,
installation, runtime-copy inventory, executable identity and real launch-path
verification remain the release orchestrator's responsibility after CI/build
validation. No production launcher, live checkout, runtime symlink, remote, helper
registration or service was changed; no push or PR was performed.
