# rust-v0.168.0 integration handoff

Source: `2bcc5f74251f845500f18ec9fa41afa78a00533d`.
`origin/main` was identical to that source; its merge was already satisfied.
Official tag: `rust-v0.168.0`, commit `9480ec100c3edee9fc69172e8e80a4730444094d`.

The merge retains upstream workspace panes, daemon recovery, Markdown question
rendering, assistant question annotations, native `/voice`, and transcription.
Custom clipboard monitoring remains alongside upstream daemon restart selection;
clipboard Markdown rendering uses the new list-spacing API. Parent-owned command
routing includes upstream Config and argument-bearing Title/Statusline handling.

The captured dirty patch predates the starting commit. Clipboard repair, sound
routing, approval/completion selection, updater isolation and release-agent wiring
were already committed in newer forms. Keep those newer implementations rather
than reinstating the old mutable updater or historical version 0.144.6. The
remaining raw-output removal is ported across settings, command routing, keymaps,
status surfaces and rendering. Related snapshots and remaining rich-rendering
regressions are adapted; tests solely exercising removed controls are removed.
Upstream's dedicated PlanModePrompt notification remains separate from interactive
UserInputRequested notifications and sounds.

All intentional untracked paths were inventoried and are present (some already
refactored into separate regression-test files or newer release-agent wrappers).
Generated `.snap.new` files and `.bazelversion 2` were excluded. Archived custom
transcription source remains inactive. No custom transcription slash command,
global capture shortcut, or helper registration is restored; `william/install`
continues to retire only its known historical helper symlinks.

## Validation in this sandbox

- `cargo metadata --no-deps --format-version 1`: passed. Workspace version is
  0.168.0; local package lock versions were aligned with manifest metadata.
- `CODEX_ROOT=<isolated clone> bash scripts/test-codex-sound-path.sh`: passed,
  including completion, approval and interactive-question routing.
- `cargo fmt -- --config imports_granularity=Item`: ran successfully using the
  available stable toolchain, which warns that imports_granularity needs nightly.
- `python3 .github/scripts/verify_cargo_workspace_manifests.py`: blocked by
  `ModuleNotFoundError: No module named 'tomllib'` in the available Python.
- `just test`, `just write-config-schema`, `just bazel-lock-update`, `just fmt`
  and scoped `just fix`: unavailable (`operation not permitted: just`). No
  replacement tools were installed. The removed schema fields were updated
  directly; canonical regeneration remains required before publication.
- `cargo shear --deny-warnings`, the focused nextest recipe equivalent
  (`-p codex-tui`, clipboard/sound/parent_owned/notifications filter), and fresh
  `cargo build -p codex-cli -p codex-tui` all stop resolving V8:
  `https://github.com/openai-oss-forks/rusty_v8.git?rev=90a65f887a0c0f048f3784afbcb320b373c8a2e0`.
  Cargo cannot create `/Users/williamxu/.cargo/git/db/rusty_v8-2d66e6cfa36ad61f`:
  `Operation not permitted (os error 1)`. Compiler state was directed to
  `/private/tmp/codex-tui-target`, with incremental compilation disabled.

This is a source-integration handoff, not a validated runtime release. The
orchestrator must rerun manifest/schema/lock generation, nightly formatting,
Clippy, focused tests and snapshots, CI and the full fresh build with dependency
access before publication. Production launchers, registered services and the live
checkout were not modified. No runtime was installed or activated, and runtime
freshness/launch-path verification remains pending with the publishing agent.
