## Symptom

The terminal launcher remains on `0.153.4` after official stable `0.154.0` was
published. The release watcher runs every 900 seconds but skips `rust-v0.154.0`
after its first failed attempt.

## Expected behavior

The launchd watcher must invoke the one active terminal Codex binary without
depending on an unrelated package-manager copy or an interactive-shell `PATH`.
After a transiently failed release is repaired and explicitly retried, the
matched runtime bundle must pass CI, activate atomically, and launch from the
normal `codex` command.

## Diagnosis

The production npm CLI was intentionally removed when the immutable local
runtime became canonical. However, the launchd plist omits `~/.local/bin` and
does not set `CODEX_RELEASE_AGENT_BINARY`. The release agent therefore used its
bare `codex` default, which launchd could no longer resolve. The ledger recorded
`[Errno 2] No such file or directory: 'codex'` for `rust-v0.154.0`; later watcher
runs correctly deduplicated that failed claim but could not self-repair it.

The first manual retry also exposed that the installed runner defaulted to the
source checkout's agent path when it ran outside launchd. That checkout is
intentionally allowed to be old and dirty, so the installed runner must default
to the installed agent copy while the repository script must continue using its
source-tree peer.

The next retry reached the canonical binary but its restricted filesystem
profile did not permit the binary's immutable release directory. Session
initialization failed when the sandbox helper attempted to re-execute Codex.
The agent must resolve an absolute launcher to its immutable target and add only
that target directory to the read profile.

## Plan

1. Resolve the release-agent binary to the canonical active launcher when it
   exists, while retaining the bare-command bootstrap fallback.
2. Give launchd an explicit canonical binary and include the local bin directory
   in its deterministic `PATH`.
3. Make the installed runner self-contained without changing source-tree runs.
4. Allow the resolved immutable binary directory in the integration agent's
   restricted read profile.
5. Add regression coverage for canonical, configured, bootstrap, installed
   runner, and sandboxed runtime resolution.
6. Pass focused updater tests and required fork CI, reinstall the watcher, and
   explicitly retry `rust-v0.154.0`.
7. Verify the signed three-binary bundle, unique shell resolution, live process
   mapping, and an end-to-end terminal launch.

## Status

Diagnosed on 2026-09-12; implementation in progress.
