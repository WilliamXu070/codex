## Symptom

Starting `codex` from the normal shell reports `0.151.0-alpha.8` instead of the current stable `0.153.4`. The canonical `codex-tui` link is broken, the companion host comes from `0.148.0-alpha.22`, and a second npm-managed CLI remains discoverable on `PATH`.

## Expected behavior

Exactly one terminal-managed `codex` installation is discoverable. `codex`, `codex-tui`, and `codex-code-mode-host` resolve to one immutable release directory, report the same release version where supported, and the real terminal launch shows `0.153.4`.

## Diagnosis

Repository instructions redirect the canonical production launchers to a mutable Cargo target after normal development builds. Later builds replaced or removed artifacts in that directory without updating the release manifest or companion host. Separately, the release ledger records transient failures as terminal and skips the same tag forever unless manually retried, so the failed `0.153.4` attempt never self-healed after its source path returned.

The fork's repository CI also pinned npm staging to an expired June release workflow, so otherwise valid updater changes could not reach the required aggregate gate.

## Plan

1. Separate development launchers from production launchers.
2. Validate and reconcile the complete runtime bundle before treating a release as current.
3. Classify preflight failures as retryable and avoid permanently claiming a tag before source validation.
4. Add regression tests for mutable-target drift, missing companions, retry recovery, and atomic rollback.
5. Integrate upstream `rust-v0.153.4`, pass required CI, build and sign all runtime artifacts, then activate them atomically.
6. Remove the redundant npm-managed terminal CLI only after the new runtime passes the exact shell-launch workflow.

## Verification

- Required fork CI is green.
- Candidate golden flows pass without touching production.
- Failure-injection tests prove partial activation cannot occur.
- `type -a codex` returns one terminal path.
- The active process resolves to the immutable `0.153.4` release.
- Two release-watch intervals produce a clean no-op with no runtime drift.

## Status

Resolved for every new terminal launch on 2026-09-07.

- Required CI passed on pull requests 33, 34, 35, 36, and 37.
- Release pull request 36 merged at `53443dac02294e7eba9809bd1328dd6dff8f21de`.
- `codex`, `codex-tui`, and `codex-code-mode-host` resolve to the immutable
  `0.153.4-53443dac0229` release directory and pass version, signature, and host
  smoke checks.
- The redundant global npm CLI was uninstalled, and the legacy standalone
  release tree was moved outside launcher discovery.
- A fresh login shell resolves exactly one terminal `codex`, and the interactive
  banner reports `0.153.4`.
- Two stable-channel watcher cycles completed as clean no-ops, and rollback plus
  drift-reconciliation regressions pass.

The TUI session that performed this repair was started from the old mutable
`0.151.0-alpha.8` executable. Its already-mapped process image remains until
that session exits; it is no longer a launchable or discoverable installation.
