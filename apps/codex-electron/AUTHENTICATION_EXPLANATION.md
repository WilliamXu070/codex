# Authentication Issue Explanation

## The Problem

You're seeing these errors in the debug stream:

```
stderr: ERROR codex_core::models_manager::manager: failed to refresh available models: unexpected status 404 Not Found
```

### Root Cause

Codex tries to refresh the list of available models from the remote API when:
1. A new Codex session starts
2. The model cache expires (every 5 minutes)
3. A new turn begins

This requires **authentication** (either an API key or ChatGPT login). Without authentication:
- The API returns `404 Not Found`
- Codex logs the error but continues working
- The app can't refresh the model list from the server
- You may be limited to locally-configured models

### Why It's Non-Fatal

The errors don't stop Codex from working because:
- Codex falls back to cached models or built-in model presets
- The error is logged but doesn't crash the session
- Commands and turns still execute successfully

However, you won't get:
- The latest model list from the server
- Access to new models as they're released
- Proper model metadata and capabilities

## Solution

I've added authentication support to the Electron app. You now have access to:

### New API Methods

```typescript
// Check authentication status
const account = await window.codexApi.codex.account.read();

// Login with API key
await window.codexApi.codex.account.loginStart({
  type: "apiKey",
  apiKey: "sk-..."
});

// Login with ChatGPT (browser flow)
const result = await window.codexApi.codex.account.loginStart({
  type: "chatgpt"
});
// Opens browser at result.authUrl, wait for account/login/completed event

// Logout
await window.codexApi.codex.account.logout();
```

### Events

The app now listens for:
- `account/login/completed` - When login finishes (success or error)
- `account/updated` - When auth status changes

## Next Steps

To fix the 404 errors, you should:

1. **Add a login UI** to your Electron app that:
   - Checks auth status on startup (`account/read`)
   - Shows login button if not authenticated
   - Handles API key or ChatGPT login flow
   - Displays current auth status

2. **Handle the login flow**:
   - For API key: prompt user for key, call `loginStart({ type: "apiKey", apiKey })`
   - For ChatGPT: call `loginStart({ type: "chatgpt" })`, open the returned `authUrl` in browser, wait for `account/login/completed` event

3. **Show auth status** in the UI so users know if they're logged in

## Example Implementation

Here's a simple example you could add to `App.tsx`:

```typescript
const [authStatus, setAuthStatus] = useState<{
  account: null | { type: string; email?: string };
  requiresAuth: boolean;
} | null>(null);

useEffect(() => {
  if (codexReady) {
    window.codexApi.codex.account.read().then(setAuthStatus);
  }
}, [codexReady]);

// In your render:
{authStatus?.requiresAuth && !authStatus?.account && (
  <div>
    <p>Authentication required to refresh model list</p>
    <button onClick={async () => {
      const result = await window.codexApi.codex.account.loginStart({ type: "chatgpt" });
      if (result.authUrl) {
        window.open(result.authUrl);
      }
    }}>
      Login with ChatGPT
    </button>
  </div>
)}
```

Once authenticated, the 404 errors should stop appearing.


