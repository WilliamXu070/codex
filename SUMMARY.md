# Codex Project Summary

## Overview
Codex is a multi-component repo that ships a Rust-based agent core (`codex-rs`) with multiple frontends and servers. This workspace now includes an Electron desktop app (`apps/codex-electron`) that provides a Finder-style file browser and a Codex prompt panel backed by the app-server JSON-RPC API.

## Key Components
- `codex-rs/`: Rust workspace containing the core agent, app-server, and supporting crates.
- `codex-cli/`: Node launcher for packaged Codex binaries (vendor-based; not used for dev builds).
- `apps/codex-electron/`: Electron + React/Vite desktop UI that runs a local Codex app-server in the background.
- `docs/`: Documentation for CLI, MCP interface, and project behaviors.

## Desktop App (Electron)
- Frontend: React + Vite (renderer) with Finder-like browsing and a bottom prompt panel.
- Backend: Electron main process spawns the Rust `codex-app-server` binary and speaks JSON-RPC over stdio.
- Debug: In-app debug stream shows build steps, requests, responses, and errors.
- Hidden files: dotfiles and Docker-related folders are suppressed in the file list.

## How To Run (Dev)
From repo root:

```bash
corepack pnpm --filter codex-electron dev
```

Notes:
- The first run builds `codex-app-server` from source and can take a while.
- Ensure Rust/Cargo is installed and visible to Electron (PATH includes `~/.cargo/bin`).

## Current Status
- App-server binary is built from `codex-rs/` and spawned directly to keep JSON-RPC output clean.
- App-server initialization handshake is required before `newConversation` and `sendUserMessage`.
- File list scrolling + chat debug stream are in place.

## Next Steps
1) Improve UI/UX usability
   - Display all the possible slash commands
      - /init /agent ...
      - Display these as options for the user to know

2) Customize the cli tooling
   - Implement subagents
      - Like claude code
   - Improve functionality for word, slides, excel file formats
      - Potentially enabling in document editing and such. like cursor but in the word document maybe
   - Include multiple models
      - Use of API's and such
      - Claude, gemini, copilot, etc...

3) Implement saving and chat history
   - Currently the chat history is not saved
   - The storing of the chat in the specific directory is not done correctly 

4) Novel features
   - Custom voice integration (like JARVIS)
      - Does tasks as conversational stuff
   - Improve upon the agents.md file stuff
