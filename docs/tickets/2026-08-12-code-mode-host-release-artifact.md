# Custom Codex releases omit the code-mode host

## Symptom

A fresh task started through `~/.local/bin/codex` cannot dispatch any code-mode
cell. The tool fails before the nested command runs:

```text
failed to spawn code-mode host /Users/williamxu/.local/bin/codex-code-mode-host
```

## Expected behavior

Every activated custom Codex release must contain a version-matched
`codex-code-mode-host`, expose it beside the active CLI, and roll all release
artifacts backward together if activation fails.

## Diagnosis

The release agent builds, validates, signs, copies, activates, and records only
`codex` and `codex-tui`. The installed `0.148.0-alpha.9` source resolves the
local host beside the running executable, but no host artifact or active symlink
exists. Stock npm and Desktop packages include the helper, confirming this is a
custom packaging failure rather than a host architecture or interpreter error.

Adding the host to the plain Cargo command exposed a second packaging defect:
the `v8` crate's default download points at a nonexistent upstream archive for
this build profile. Codex's supported build path resolves a matching archive and
generated bindings from the `openai/codex` V8 release and verifies both against
the published checksum manifest. The updater must run that resolver before
Cargo, not rely on the crate's fallback URL.

## Plan

1. Build `codex-code-mode-host` with the CLI and TUI.
2. Resolve the official checksum-verified V8 archive and bindings before Cargo.
3. Validate the host executable and stdio startup contract.
4. Copy and sign it with the upstream macOS JIT entitlements.
5. Activate and roll back CLI, TUI, host, and current-release links as one unit.
6. Record the host and previous host in release metadata.
7. Add regression coverage for build selection, V8 setup, permanent installation,
   activation, metadata, and rollback.
8. Repair the active alpha.9 installation from its exact merge commit and run a
   real custom-Codex code-mode command.

## Verification

- All 35 focused release-agent tests pass, including missing-host rejection and
  four-link rollback on a forced host smoke-test failure.
- `just fmt`, Ruff formatting, Ruff lint, and `git diff --check` pass.
- The host builds from merge commit
  `a7fdd7aa366596af3ce16b5dad63c0bdbc8acdb6` after resolving Codex's official
  checksum-verified V8 artifact pair.
- The active host is executable, signed with the configured local identity, and
  carries `allow-jit` plus `allow-unsigned-executable-memory` entitlements.
- A fresh `~/.local/bin/codex exec` task with code-mode-only enabled dispatched
  `pwd` and returned `CODE_MODE_HOST_OK` without the spawn error.
- The original dirty checkout was not modified.

## Status

Fixed and verified on 2026-08-12.
