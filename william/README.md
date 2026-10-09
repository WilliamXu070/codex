# William Codex

One folder for personal Codex extensions.

```text
william/
  audio/      Codex completion and attention sounds
  commands/   Sound slash-command helper
  transcribe/ Archived custom voice helpers (not installed)
  install     Links this folder into ~/.codex
```

Current commands:

```sh
~/.codex/commands/sound status
~/.codex/commands/sound volume 30
~/.codex/commands/sound mute
~/.codex/commands/sound unmute
~/.codex/commands/sound profile quiet
~/.codex/commands/sound test
~/.codex/commands/sound track list
~/.codex/commands/sound track set 01-kanye-west-wolves-meme.mp3
~/.codex/commands/sound track random
```

Install:

```sh
./william/install
```

The TUI retains `/sound`, completion sounds, and approval sounds. Use native
upstream `/voice` for voice conversations and the native transcription workflow.
The custom `/transcribe` and `/transcribe-command` menus and Ctrl+Shift+D capture
(including the collapsed Ctrl+D fallback) are retired. Installation removes only
registered custom voice helper symlinks and preserves existing sound settings.
Archived voice helper source remains available for reference.
