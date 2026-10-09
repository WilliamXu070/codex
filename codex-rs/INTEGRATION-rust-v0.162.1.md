# Codex 0.162.1 integration handoff

Branch: `agent/upstream-0.162.1`.
Starting fork and local `origin/main`: `654f29f0591b09017f1a5be7fbe0305f58b7b384`.
Official tag object: `7a078f6bb4ce0dc2a81408b334ce52979d92c603`.
Official release commit: `092d3acd6bec3e3a14bdc7e7a2810ab628ab759d`.

The `origin/main` merge was already up to date. Merge commit `ef9677e2e`
retains both the fork and exact official release as ancestors. Its only conflict
was the workspace version, resolved to 0.162.1. Both actual upstream fixes are
retained: multiline async questions preserve line breaks and hyperlink targets,
and daemon feature compatibility uses explicit CLI overrides. The affected
implementations and upstream regression tests match the tag exactly.

## Saved changes audit

Inspected every file in `.codex-release-context/dirty.patch`, attempted
`git apply --3way`, and compared individual patches with the merged source.
The patch targets 0.141.0/0.144.6, while the starting fork already contains later
ports and retirement work. Kept current implementations rather than duplicating
old inline methods or reverting upstream APIs.

- Build instructions and external Cargo target defaults already match the saved
  intent. Production activation remains exclusive to the release agent.
- Clipboard repair is wired into `app/startup.rs`, with silent native clipboard
  initialization in `clipboard_copy/macos.rs`, canonical Markdown extraction,
  merged NSPasteboard dependency features, and extracted regression tests.
- `/sound` opens the local settings popup. User questions retain their dedicated
  notification and immediate sound route; completion and approval helper behavior
  includes the saved repository-relative Wilhelm scream fallback.
- The saved raw-mode removals were explicitly superseded during the 0.162.0
  integration (see its handoff). Retained the newer transcript/raw-mode,
  configuration, status-surface, streaming and keymap architecture and tests.
  Clipboard repair remains active for rich output. Did not revert this newer
  intentional behavior to an older patch's architecture.
- The saved custom capture RMS and braille changes belong to retired custom voice
  capture. Did not restore capture state/events, marker methods, transcription
  menus, global capture bindings, Ctrl+Shift+D or Ctrl+D capture fallbacks.
  Removed a leftover unused `TuiGlobalKeymap.transcribe` field and its three schema
  entries. The resulting config schema is identical to the official tag.
  Native `/voice` and upstream transcription code remain intact.
- Retained the newer durable release-agent updater and watcher rather than the
  historical live-checkout stash/pull/build/symlink updater. Webhook validation,
  release ledger deduplication and isolated builds remain in place.
- Reconciled historical package-version edits to 0.162.1. All 166 workspace
  packages and their Cargo.lock entries now agree; external dependency versions
  and resolutions are unchanged.

Compared all eight intentional `untracked.json` entries with tracked successors:
clipboard repair and extracted tests; three incident documents; webhook server
and installer; sound regression script; archived live WAV recorder. They are
already retained. Documentation differences reflect newer integration status;
webhook differences implement the durable release agent; recorder differences
are formatting and removal of obsolete Python compatibility syntax. No missing
source required copying. Excluded every generated `.snap.new` and `.bazelversion 2`.
Archived transcription source remains inactive; `william/install` registers only
sound helpers and retires recognized custom voice symlinks.

## Validation and delivery boundary

All Cargo invocations use `/private/tmp/codex-tui-target` with incremental state
disabled. No replacement tools were installed.

Passed:

- Dependency-free Cargo metadata: all 166 workspace packages report 0.162.1.
- `cargo fmt -- --config imports_granularity=Item`: successful; stable rustfmt
  warns that imports granularity requires nightly.
- `CARGO_NET_OFFLINE=true cargo shear --deny-warnings`: no issues found.
- Required `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`:
  completion, approval and user-input paths pass without playing audio.
- 65 focused TUI nextest tests: all passed, including all six clipboard repair
  tests, local `/sound` popup routing, question notifications and clipboard
  routing/ownership regressions.
- Five release-webhook tests and one watcher-install test.
- Bash/Zsh syntax checks for updater, watcher, webhook, sound and archived helpers.
- Config schema JSON parses and equals upstream; `git diff --check` passes.

Environment limitations:

- `python3 .github/scripts/verify_cargo_workspace_manifests.py` fails with
  `ModuleNotFoundError: No module named 'tomllib'` under the available Python 3.9.
- Initial online `cargo shear --deny-warnings` cannot download
  `https://index.crates.io/config.json` for `futures` in `codex-git-utils`:
  `[6] Couldn't resolve host name (Could not resolve host: index.crates.io)`.
  Retrying against cached dependencies offline passes.
- `/opt/homebrew/bin/just` is blocked with `operation not permitted`, preventing
  canonical `just fmt`, focused `just test`, `just write-config-schema` and
  `just bazel-lock-update`. The schema was reconciled directly with upstream.
  No external dependency changes were made; the orchestrator must regenerate
  and check the Bazel lockfile before publication.
- `codex-config` nextest: 366 passed, one failed.
  `thread_config::remote::tests::load_thread_config_calls_remote_service` fails
  at `config/src/thread_config/remote.rs:400` during mock-server setup:
  `bind test server: Os { code: 1, kind: PermissionDenied, message: "Operation not permitted" }`.
  Compilation succeeds; retry this unchanged socket-dependent test with normal
  local network permissions. No test was weakened or removed.
- Focused tests use the existing just recipe's nextest command with
  `RUST_MIN_STACK=8388608` and `NEXTEST_PROFILE=local`; the TUI and config results are recorded above.

This is a source integration handoff. Runtime delivery is incomplete until the
orchestrator performs the full current-source build, immutable release install,
artifact inventory/uniqueness checks, runtime identity proof and actual production
launcher verification. Those operations are outside this task's authorized clone
and sandbox. No live checkout, production launcher, binary symlink, installed
helper, registration or runtime process was changed. No push, PR, remote change
or activation occurred. Release context remains intact and ignored.
