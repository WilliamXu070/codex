# Codex Desktop (Electron)

This app runs the Codex MCP server in the background and exposes a Finder-like file manager plus a Codex prompt panel.

## Dev

From repo root:

```bash
pnpm install
pnpm --filter codex-electron dev
```

By default the app builds and runs the app server from source:

```bash
cargo build -p codex-app-server
./codex-rs/target/debug/codex-app-server
```

If you want a different command, edit the Command/Args fields in the UI or set `CODEX_REPO_ROOT`.

## Build (renderer only)

```bash
pnpm --filter codex-electron build
```
