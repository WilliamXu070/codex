# William Codex

One folder for personal Codex extensions.

```text
william/
  audio/      Codex completion and attention sounds
  commands/   Future slash-command executables
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
~/.codex/commands/dictate start
/transcribe
/transcribe set-provider openai
/transcribe set-key sk-...
/transcribe set-language en
```

Install:

```sh
/Users/williamxu/Desktop/Projects/codex/william/install
```

The Codex binary stays clean. Codex only calls the scripts already configured in
`~/.codex/config.toml`.
