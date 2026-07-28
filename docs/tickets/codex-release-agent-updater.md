# Codex release updater must integrate upstream through one deduplicated agent run

## Symptom

The local updater rebuilds the dirty custom checkout at `0.144.6` instead of integrating the detected OpenAI release. Polling or duplicate webhook deliveries can also trigger repeated work.

## Expected behavior

A published `openai/codex` release should claim its tag exactly once, launch one isolated Codex integration run, preserve the custom fork and dirty local edits, verify the requested version and custom features, then push a branch and open a draft PR. The live checkout must remain untouched.

## Diagnosis

- The updater watches `origin`, which is the personal fork rather than `openai/codex`.
- A detected tag is used only as a rebuild signal; it is never fetched or merged.
- The live branch is divergent and `git pull --ff-only` cannot resolve it.
- The dirty-tree guard aborts normal updates, while forced runs only rebuild the stale tree.
- The release watcher/webhook has no durable per-tag claim, so delivery retries are not safely idempotent.

## Plan

1. Filter webhook input to published releases from `openai/codex` and valid Rust release tags.
2. Add a SQLite ledger keyed by repository and release tag, plus a process lock.
3. Clone the custom checkout into an isolated release workspace, fetch the personal fork and exact upstream tag, and give one `codex exec` run the dirty patch as read-only context.
4. Require the agent to merge both `origin/main` and the upstream tag, preserve custom functionality, test, and commit.
5. Independently validate ancestry, version, clean state, and targeted tests before pushing and opening a draft PR.
6. Add explicit manual retry; never retry failed, running, or successful tags automatically.

## Verification

- Unit tests prove wrong events and duplicate deliveries do not launch an agent.
- A live run for `rust-v0.146.0-alpha.14` integrates the dirty checkout in isolation.
- The original checkout fingerprint remains unchanged.
- The resulting branch contains the upstream tag, reports `0.146.0-alpha.14`, preserves custom sound/transcription tests, pushes, and has a draft PR.

## Status

In progress.
