# Codex 0.164.0 integration handoff

Branch: `agent/upstream-0.164.0`.
Starting personal source: `2bcc5f74251f845500f18ec9fa41afa78a00533d`.
`origin/main` already points to that source; merging it was a no-op.
Official annotated tag: `rust-v0.164.0` (tag object
`fe3b29455488c2e189e43ad5526a107900c5e1b7`), release commit
`6aaac694803004a63c6f393d153c96d22521a6b9`.
Both histories are ancestors of the integrated branch.

## Conflict decisions

- Adopt upstream daemon restart/recovery, removed-feature handling, explicit
  feature overrides, and its public-API regression tests.
- Adopt upstream Markdown async-question rendering and its multiline/link
  snapshot and tests; preserve the fork's clipboard repair polling alongside
  upstream daemon restart polling in the startup event loop.
- Preserve the extracted parent-owned-command module and update its allowlist
  to upstream's current behavior (Config and argument-bearing Title/Statusline).
- Retain both custom-settings and upstream quick-dials test modules.
- Preserve custom workspace crates; reconcile local lockfile package versions
  against Cargo metadata for all 171 workspace packages. Remove the obsolete
  shell-escalation lock entry removed by upstream.

## Dirty release context accounting

`dirty.patch` was applied with three-way merging, excluding the removed old
command-popup snapshot path. The context predates the starting commit: its
workspace version is 0.144.6 and much of its work was already integrated.
Intent was checked against the newer source rather than duplicating old hunks.

- Clipboard repair, macOS stderr suppression, canonical assistant copy lines,
  regression tests, and startup polling already exist in extracted modules.
  Preserve those implementations and their tests, including raw output mode,
  rather than apply old deletions or duplicate methods.
- Request-user-input sound dispatch already handles both legacy questions and
  asynchronous questions. Preserve that broader implementation and random
  completion/approval routing, `/sound`, and the audio helper scripts.
- Preserve newer status-line variants, keymaps, raw-output schema and tests;
  historical deletions would remove intentional later fork features.
- Preserve the current release-agent updater and launchd-safe installation,
  immutable-release activation, stable-channel default, and cache placement.
  The dirty updater's old stash/build/symlink flow is superseded by this agent
  architecture and must not replace it.
- Keep current AGENTS runtime restrictions and external compiler-state settings.
  Do not restore old direct production launcher replacement instructions.
- Reject retired capture constants, events, waveform methods, tests and keymaps.
  Remove the stale composer doc comment describing that retired capture.
  Native upstream `/voice` and voice transcription remain intact.
- The remaining thread-manager sample, sound-state, helper and formatting edits
  are already incorporated in current source.

Every intentional entry in `untracked.json` already exists in the starting
history: clipboard source and regression tests, three ticket documents, webhook
server/installer, sound regression script, and archived recorder helper.
Keep the newer formatted and tested copies instead of overwriting with older
context copies. Archived transcription scripts remain source only;
`william/install` registers sound helpers and removes only known retired helper
symlinks. It does not register dictation/transcription helpers.
Exclude all context `*.snap.new` files and `.bazelversion 2`.
Release context is preserved, unmodified and uncommitted.

## Validation in the restricted sandbox

Passed:

- `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`.
- Release-watch installer tests: 1 passed; webhook server tests: 5 passed.
- Shell syntax checks for updater, watcher installer, webhook installer and
  sound-path regression script.
- Cargo metadata with installed Rust 1.95.0, no dependencies, offline: all 171
  workspace packages parsed; workspace version is 0.164.0.
- Rust formatting from `codex-rs` with installed Rust 1.95.0 completed.
  It warns that `imports_granularity=Item` requires nightly, so the orchestrator
  must repeat formatting with the pinned toolchain.
- `git diff --check` and ancestry checks for both required histories.

Blocked (no replacement tools installed):

- `python3 .github/scripts/verify_cargo_workspace_manifests.py`: available
  Python 3.9 lacks `tomllib`. Release-agent and retention tests have the same
  import blocker; the other six Python tests pass.
- Pinned Rust 1.99.1 is not installed. Rustup attempts to sync it, then fails
  creating `/Users/williamxu/.rustup/tmp/...`: Operation not permitted.
- `just fmt`, focused `just test -p codex-tui clipboard_repair`,
  `just fix -p codex-tui`, and `just bazel-lock-update`: execution of `just`
  is denied by the sandbox. The existing tool is outside permitted paths.
- `cargo shear --deny-warnings` using installed Rust 1.95.0 reaches metadata
  resolution but fails fetching `os_info` for `codex-analytics`: downloading
  `https://index.crates.io/config.json` fails because index.crates.io cannot
  resolve. It does not complete dependency analysis.
- Focused nextest recipe equivalent (clipboard repair, sound, question rendering,
  daemon feature tests), with offline installed Rust: metadata cannot check out
  uncached `rmcp` from `https://github.com/modelcontextprotocol/rust-sdk`, revision
  `98a54789b80110ad210495c4ad845c89e1980471`. No Rust tests executed.

## Publication boundary

This is a source integration handoff, not a deployed runtime fix. No live
checkout, production launcher, binary symlink, service registration, or remote
was changed; no push or PR was made. Build, Bazel lock regeneration, pinned
formatting, Rust tests and runtime freshness/uniqueness verification remain for
orchestrator validation before publication. The intended runtime artifacts are
immutable release copies of codex, codex-tui and codex-code-mode-host activated
through the production launchers; this worker was explicitly not authorized to
refresh or activate them.
