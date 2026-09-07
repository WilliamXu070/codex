## Symptom

Starting `codex` from the normal shell reports `0.151.0-alpha.8` instead of the current stable `0.153.4`. The canonical `codex-tui` link is broken, the companion host comes from `0.148.0-alpha.22`, and a second npm-managed CLI remains discoverable on `PATH`.

## Expected behavior

Exactly one terminal-managed `codex` installation is discoverable. `codex`, `codex-tui`, and `codex-code-mode-host` resolve to one immutable release directory, report the same release version where supported, and the real terminal launch shows `0.153.4`.

## Diagnosis

Repository instructions redirect the canonical production launchers to a mutable Cargo target after normal development builds. Later builds replaced or removed artifacts in that directory without updating the release manifest or companion host. Separately, the release ledger records transient failures as terminal and skips the same tag forever unless manually retried, so the failed `0.153.4` attempt never self-healed after its source path returned.

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

In progress.
