# Release automation must not load personal credentials or publish invalid integrations

## Symptom

The unattended `0.147.0-alpha.2` release run loaded the personal `mhacks` MCP,
refreshed its OAuth token in macOS Keychain, and caused Codex Desktop to request
Keychain approval. The generated PR also failed `Fork CI / Bazel analysis`
because `defs.bzl` contained the same function parameter twice.

## Expected behavior

Release integration and repair agents must use Codex authentication without
loading personal MCP servers, plugins, hooks, or other user configuration. The
generated branch must pass the exact fork CI gate before merge or activation,
and the dirty live checkout must remain unchanged.

## Diagnosis

- `run_codex_agent()` invoked `codex exec` with the normal user configuration,
  so every background integration initialized `~/.codex/config.toml` MCPs.
- The `mhacks` token was refreshed and rewritten immediately before Codex
  Desktop accessed the same Keychain item.
- The release merge retained both the fork and upstream declarations of
  `binary_test_target_compatible_with`, which made `defs.bzl` fail to parse.
- The CI gate failed closed, so `0.147.0-alpha.2` was not merged or activated.

## Plan

1. Invoke unattended Codex agents with `--ignore-user-config` while retaining
   explicit release permissions and normal `CODEX_HOME` authentication.
2. Add a command-construction regression test for the isolation flag.
3. Remove the duplicate Bazel parameter and documentation entry.
4. Reproduce the exact Python updater tests and Bazel queries locally.
5. Push the focused repair to PR #5 and require `CI required` to pass before
   retrying merge and activation.
6. Verify the active CLI and dirty live-checkout fingerprints after activation.

## Verification

- `python3.14 -m unittest discover -s scripts/tests -p 'test_codex_release_*.py'`:
  19 tests passed.
- `bazel query //codex-rs/windows-sandbox-rs:all`: passed under Bazel 9.0.0.
- `bazel query //codex-rs/context-files:all`: passed under Bazel 9.0.0.
- `just fmt`: passed without additional changes.
- A real `codex exec --ignore-user-config --ephemeral` probe returned
  `ISOLATION_OK`; its log window contained no non-desktop
  `codex_keyring_store` events.
- Full PR `CI required` verification is pending.

## Status

Local verification complete; awaiting PR CI.
