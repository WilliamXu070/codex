# Slash Commands Implementation

## The Problem

When you typed `/init` in the Electron app, it was being sent as literal text to the model instead of being handled as a special command. This is why:

1. The model received the text "/init" instead of the special prompt
2. AGENTS.md wasn't being generated correctly
3. The file existence check wasn't happening

## How It Works in the TUI

In the Codex TUI (terminal UI), slash commands are intercepted **before** sending to the model:

1. User types `/init`
2. TUI detects it's a slash command
3. TUI checks if `AGENTS.md` exists in the current working directory
4. If it exists: Shows a message and skips
5. If it doesn't exist: Sends a special prompt (from `prompt_for_init_command.md`) instead of the literal "/init" text

## The Fix

I've implemented slash command handling in the Electron app:

### 1. Added File Check API

```typescript
// Check if AGENTS.md exists
const exists = await window.codexApi.fs.checkAgentsMd(currentPath);
```

### 2. Intercept `/init` Before Sending

In `handleSend()`, the app now:
- Detects if the input starts with `/`
- For `/init`:
  - Checks if `AGENTS.md` exists
  - If exists: Shows message and returns
  - If not: Sends the special init prompt instead of "/init"

### 3. The Special Prompt

The `/init` command sends this prompt to the model:

```
Generate a file named AGENTS.md that serves as a contributor guide for this repository.
[... full prompt from prompt_for_init_command.md ...]
```

This tells the model to:
- Analyze the repository structure
- Generate an AGENTS.md file
- Include sections for project structure, build commands, coding style, testing, etc.

## Current Status

✅ `/init` command is now properly handled
- Checks for existing AGENTS.md
- Sends the correct prompt
- Model will generate the file

⚠️ Other slash commands (like `/model`, `/review`, etc.) show a "not implemented" message. These would need similar handling if you want to support them.

## Testing

1. Navigate to a directory without AGENTS.md
2. Type `/init` and send
3. The model should generate an AGENTS.md file
4. Try `/init` again - it should show "AGENTS.md already exists" message

## Future Enhancements

To fully support all slash commands, you'd need to:

1. Parse all slash commands (see `codex-rs/tui/src/slash_command.rs`)
2. Handle each command appropriately:
   - `/model` - Open model selection UI
   - `/review` - Trigger review flow
   - `/new` - Start new session
   - `/resume` - Resume old session
   - etc.

For now, `/init` is the most critical one for repository setup.


