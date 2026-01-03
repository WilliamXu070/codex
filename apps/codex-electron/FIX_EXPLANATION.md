# Electron App Codex Integration Fix

## Problem Analysis

The Electron application was not working when trying to prompt Codex through the CLI because of a **protocol mismatch**:

1. **Binary Used**: The app was correctly spawning `codex-app-server` (the app-server binary)
2. **API Used**: However, it was trying to use **MCP-style methods** (`newConversation`, `sendUserMessage`)
3. **Expected API**: The app-server actually expects **app-server API methods** (`thread/start`, `turn/start`)

### The Two Different APIs

Codex has two different server interfaces:

1. **MCP Server** (`codex mcp-server`):
   - Uses methods like `newConversation`, `sendUserMessage`
   - Designed for MCP (Model Context Protocol) clients
   - Protocol: JSON-RPC 2.0 with `jsonrpc: "2.0"` field

2. **App Server** (`codex app-server`):
   - Uses methods like `thread/start`, `turn/start`
   - Designed for rich desktop/IDE integrations (like VS Code extension)
   - Protocol: JSON-RPC 2.0 but **omits** the `jsonrpc: "2.0"` field
   - Requires `initialize` + `initialized` notification handshake

The Electron app was mixing these: using app-server binary but MCP methods.

## Solution

Updated `electron/main.js` to use the correct app-server API:

### Key Changes

1. **Renamed class**: `McpClient` → `AppServerClient` (for clarity)

2. **Initialization Flow**:
   ```javascript
   // OLD (MCP-style):
   await mcpClient.request("initialize", {...});
   // Missing initialized notification!
   
   // NEW (app-server):
   await appServerClient.request("initialize", {...});
   appServerClient.notify("initialized", {}); // Required!
   ```

3. **Starting Conversations**:
   ```javascript
   // OLD (MCP-style):
   await mcpClient.request("newConversation", {
     model: options?.model,
     cwd: options?.cwd,
     ...
   });
   
   // NEW (app-server):
   await appServerClient.request("thread/start", {
     model: options?.model,
     cwd: options?.cwd,
     ...
   });
   ```

4. **Sending Messages**:
   ```javascript
   // OLD (MCP-style):
   await mcpClient.request("sendUserMessage", {
     conversationId: mcpClient.conversationId,
     items: [{ type: "text", text }]
   });
   
   // NEW (app-server):
   await appServerClient.request("turn/start", {
     threadId: appServerClient.threadId,
     input: [{ type: "text", text }]
   });
   ```

5. **Event Handling**:
   - App-server uses different event format: `item/agentMessage/delta`, `item/started`, `item/completed`, `turn/completed`
   - Converted these to the existing `codex:event` format for UI compatibility

6. **Approval Handling**:
   - App-server sends approval requests as JSON-RPC requests (with `id`)
   - Methods: `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`
   - Response format: `{ id, result: { decision: "accept"|"decline", acceptSettings? } }`

7. **Protocol Format**:
   - Removed `jsonrpc: "2.0"` field from requests (app-server omits it)
   - Kept JSON-RPC 2.0 structure otherwise

## Testing

To test the fix:

1. Start the Electron app: `npm run dev` (in `apps/codex-electron`)
2. The app should automatically build `codex-app-server` if needed
3. Enter a prompt in the chat interface
4. The app should:
   - Initialize the app-server
   - Create a thread
   - Start a turn with your message
   - Stream agent responses via `item/agentMessage/delta` events
   - Show approvals when needed

## Event Flow

```
User sends message
  ↓
App calls codex:send
  ↓
Main process calls turn/start
  ↓
App-server processes turn
  ↓
Events stream:
  - turn/started
  - item/started (agentMessage)
  - item/agentMessage/delta (streaming text)
  - item/completed
  - turn/completed
  ↓
UI updates via codex:event IPC
```

## Additional Notes

- The app-server binary is built automatically if missing
- Thread ID is stored in `appServerClient.threadId` (replaces `conversationId`)
- All events are converted to the existing `codex:event` format to maintain UI compatibility
- Approval requests are properly handled with the correct response format


