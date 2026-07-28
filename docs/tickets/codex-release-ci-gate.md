# Codex release agent must gate merge and activation on fork-compatible CI

## Symptom

The `0.146.0-alpha.14` release agent merged and activated a locally successful
Cargo build while the pull-request and postmerge GitHub workflows were failing.
The active CLI worked, but `main` remained red and GitHub sent a failed
`postmerge-ci` notification.

## Expected behavior

Each official release tag should start at most one isolated integration agent.
The agent may publish a pull request, but it must not merge or activate the
result until the fork's required CI check succeeds. Fork CI must use runners
available to `WilliamXu070/codex` and still validate formatting, Cargo
dependency hygiene, the CLI/TUI build, updater tests, and the Bazel packages
that previously failed analysis.

## Diagnosis

- `scripts/codex-release-agent.py` called `gh pr merge` immediately after
  creating the integration pull request and never inspected `CI required`.
- The integrated context crates had unused dependencies, enabled empty
  doctests, and import formatting that differed from CI's Rust 1.95 command.
- `codex-rust-crate` did not accept the
  `binary_test_target_compatible_with` argument used by
  `windows-sandbox-rs`; this is also tracked upstream as
  `openai/codex#35683`.
- `context-files/BUILD.bazel` supplied fixture files twice to generated test
  targets.
- A fixture manifest was incorrectly treated as a workspace crate, `OT` was
  treated as a spelling error, and `codex-embeddings` directly owned the
  workspace-banned `reqwest` dependency.
- OpenAI's workflows reference private runner groups and paid macOS capacity
  that the personal fork does not have, so those jobs cannot be a meaningful
  required gate in the fork.

## Plan

1. Repair Cargo, formatting, Bazel, manifest-policy, spelling, and HTTP-client
   integration failures.
2. Add a fork CI profile using available Ubuntu runners and route blocking and
   postmerge workflows to it outside `openai/codex`.
3. Poll the pull request's `CI required` check and fail closed on failure,
   cancellation, absence, or timeout before calling `gh pr merge`.
4. Add regression tests proving a failed or missing check prevents merge and
   activation while a successful check permits each exactly once.
5. Run the exact failed checks locally, open a repair pull request, wait for the
   GitHub gate, merge only after success, and verify the subsequent `main`
   workflows.

## Verification

- `cargo +1.95.0 fmt -- --config imports_granularity=Item --check`
- `cargo shear --deny-warnings`
- `cargo build -p codex-cli -p codex-tui`
- Bazel analysis for `windows-sandbox-rs` and `context-files`
- updater and CI-policy Python tests
- repair pull request reports `CI required` success before merge
- postmerge workflow succeeds on the merged commit
- active CLI remains `0.146.0-alpha.14`
- live dirty checkout fingerprint remains unchanged

## Status

Local verification complete. Awaiting the repair pull request's `CI required`
gate before merge and runtime installation.
