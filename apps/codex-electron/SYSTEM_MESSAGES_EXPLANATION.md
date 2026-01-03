# SYSTEM Messages Issue - Explanation and Fix

## The Problem

You were seeing empty "SYSTEM" messages appearing in the chat interface between user messages and assistant responses. These weren't showing up in the debug stream, making them hard to diagnose.

## Root Cause

The issue was caused by **unhandled app-server events** being forwarded to the UI:

1. **App-server sends many event types**: When Codex processes a turn, it emits various `item/started` and `item/completed` events for different item types:
   - `userMessage` - The user's input
   - `agentMessage` - The assistant's response
   - `reasoning` - Internal reasoning (shouldn't be visible)
   - `commandExecution` - Command execution status
   - `fileChange` - File modification proposals
   - `mcpToolCall` - MCP tool calls
   - etc.

2. **Incomplete event handling**: The Electron app was only handling a few event types (`agentMessage`, `exec_command_output_delta`, `error`, `task_complete`). All other events were being forwarded via a catch-all handler.

3. **Empty messages created**: When unhandled events were forwarded, they might have been creating messages with role "system" but empty or minimal text, which appeared as just "SYSTEM" in the UI.

## The Fix

I've made several improvements:

### 1. Better Event Filtering

- **Stopped forwarding unhandled notifications**: The catch-all handler now logs unhandled events to the debug stream instead of forwarding them as UI events.
- **Added logging for item lifecycle**: `item/started` and `item/completed` events for non-agentMessage items are now logged to the debug stream with descriptive messages.

### 2. Message Filtering

- **Filter empty messages**: The UI now filters out messages with empty or whitespace-only text before rendering.
- **Prevent rendering issues**: This ensures that even if an empty message somehow gets created, it won't appear in the chat.

### 3. Better Debug Visibility

- **All unhandled events logged**: You'll now see messages like:
  - `item_started: Item started: reasoning (id: ...)`
  - `item_completed: Item completed: fileChange (id: ...)`
  - `unhandled_notification: turn/plan/updated`
- **Unhandled event types logged**: In the React component, unhandled event types are logged to the debug stream.

## What You'll See Now

### In Debug Stream
You'll see more detailed logging:
```
item_started: Item started: reasoning (id: item_123)
item_started: Item started: fileChange (id: item_456)
item_completed: Item completed: commandExecution (id: item_789)
unhandled_notification: turn/plan/updated
```

### In Chat UI
- No more empty SYSTEM messages
- Only meaningful messages are displayed
- All messages have non-empty text

## Why They Weren't in Debug Before

The original catch-all handler was forwarding events directly to the UI without logging them first. The debug stream only showed events that went through the `emitStatus` method, so unhandled events were invisible.

## Future Improvements

To fully support all app-server features, you could:

1. **Handle reasoning items**: Show reasoning summaries in a collapsible section
2. **Handle file changes**: Show diffs inline
3. **Handle command execution**: Show command status and output in a structured way
4. **Handle turn plans**: Show the agent's plan as it develops

For now, these are logged but don't create UI clutter.


