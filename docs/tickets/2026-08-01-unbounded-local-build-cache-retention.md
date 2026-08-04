# Unbounded local build-cache retention

## Symptom

Local Codex, Newmux/Ghostty, Education, and Clicky development consumed hundreds of gigabytes even though the installed applications require only a small fraction of that space. iCloud Desktop sync then processed the repository-local artifacts again.

## Expected behavior

- Installed releases contain only runtime artifacts and a bounded rollback copy.
- Codex compiler intermediates never persist inside the Desktop-backed checkout or release directory.
- Ghostty local Zig caches are disposable and never persist inside the Desktop-backed checkout or worktrees.
- Ghostty application backups are retained with a fixed upper bound.
- Education packages Codex and builds Clicky through disposable compiler directories while retaining only the final runtime/app artifacts.
- Clicky release builds keep the final release artifact but remove Xcode intermediates.

## Diagnosis

- The legacy Codex updater used a unique `releases/<timestamp>` directory as `CARGO_TARGET_DIR`, retaining a complete 12-14 GB compiler tree per run.
- Direct Codex builds/tests left 61 GB in `codex-rs/target`, dominated by incremental and dependency artifacts.
- The hardened Codex release agent retained five integration workspaces after terminal success/failure states; their `codex-rs/target` trees consumed another 81 GB.
- `scripts/build-ghostty.sh` invoked `zig build` without `--cache-dir`, retaining 44 GB in `ghostty-src/.zig-cache`; two worktrees retained another 6.4 GB.
- Ghostty Spotlight refreshes accumulated unbounded application backups.
- Education's Codex packaging retained 5.3 GB under `upstream/codex-reference/codex-rs/target`, while its Clicky build retained repo-local Xcode and SwiftPM intermediates.
- Clicky Xcode builds retained about 1.6 GB of DerivedData outside the source checkout.

## Plan

1. Preserve the active Codex release, one rollback release, `/Applications/Ghostty.app`, and all source changes.
2. Make the legacy updater build through a disposable target directory, make future launchd installs use the hardened agent, bound installed releases, and remove integration workspaces on every terminal outcome.
3. Make Ghostty builds use a temporary local Zig cache and clean the selected Xcode configuration before rebuilding.
4. Bound Spotlight application backups.
5. Route Education's Codex packaging, fake runtime, and Xcode build through disposable locations while copying only final artifacts to stable output paths.
6. Route Clicky release DerivedData through a disposable location.
7. Remove only verified generated artifacts.

## Verification

- Codex retention suite: `6 passed`, including crash-leftover workspace pruning; installed CLI/TUI both report `0.147.0-alpha.2`; launchd points to the hardened installed agent.
- Changed shell and Python sources pass syntax checks; the Codex `justfile` parses and scoped diffs pass whitespace checks.
- Targeted Ghostty Zig test passed. A full clean ReleaseFast/ReleaseLocal build passed, code signature verification passed, and the built binary reports Ghostty `1.3.2`. No repo `.zig-cache`, `zig-out`, temporary build root, or global Ghostty DerivedData remained.
- Newmux Spotlight install was exercised against isolated application/config/runtime roots; config validation passed and backup pruning retained exactly two apps.
- Education smoke checks passed, including bundled checksums, Swift parsing, fake app-server protocol, and credential scan. The full clean Debug build and deep code-signature verification passed; only the 456 MB final app remains.
- Public Clicky's release script passes syntax/whitespace checks and an isolated pre-release abort test proved its temporary build root is removed. The publishing/notarization path was not executed because it is irreversible.
- The verified deletion set was 336.44 GiB logically. APFS reported free space increasing by 195.21 GiB after rebuilding and retaining the final Ghostty and Education Clicky apps.

## Status

Resolved on 2026-08-01.
