const { contextBridge, ipcRenderer } = require("electron");

contextBridge.exposeInMainWorld("codexApi", {
      fs: {
        listDirectory: (dirPath) => ipcRenderer.invoke("fs:list", dirPath),
        searchDirectory: (payload) => ipcRenderer.invoke("fs:search", payload),
        getFavorites: () => ipcRenderer.invoke("fs:favorites"),
        pickFolder: () => ipcRenderer.invoke("fs:pick-folder"),
        openPath: (targetPath) => ipcRenderer.invoke("fs:open", targetPath),
        revealInFinder: (targetPath) => ipcRenderer.invoke("fs:reveal", targetPath),
        openWith: (targetPath) => ipcRenderer.invoke("fs:open-with", targetPath),
        checkAgentsMd: (dirPath) => ipcRenderer.invoke("fs:checkAgentsMd", dirPath)
      },
      codex: {
        start: (options) => ipcRenderer.invoke("codex:start", options),
        send: (payload) => ipcRenderer.invoke("codex:send", payload),
        approve: (payload) => ipcRenderer.invoke("codex:approve", payload),
        stop: () => ipcRenderer.invoke("codex:stop"),
        account: {
          read: (params) => ipcRenderer.invoke("codex:account:read", params),
          loginStart: (params) => ipcRenderer.invoke("codex:account:login:start", params),
          loginCancel: (params) => ipcRenderer.invoke("codex:account:login:cancel", params),
          logout: () => ipcRenderer.invoke("codex:account:logout")
        },
        onEvent: (handler) => {
          ipcRenderer.on("codex:event", (_, payload) => handler(payload));
          return () => ipcRenderer.removeAllListeners("codex:event");
        },
        onApproval: (handler) => {
          ipcRenderer.on("codex:approval", (_, payload) => handler(payload));
          return () => ipcRenderer.removeAllListeners("codex:approval");
        },
        onStatus: (handler) => {
          ipcRenderer.on("codex:status", (_, payload) => handler(payload));
          return () => ipcRenderer.removeAllListeners("codex:status");
        },
        onReady: (handler) => {
          ipcRenderer.on("codex:ready", (_, payload) => handler(payload));
          return () => ipcRenderer.removeAllListeners("codex:ready");
        }
      },
      context: {
        indexDirectory: (path) => ipcRenderer.invoke("context:index-directory", path),
        queryContext: (query, maxResults = 50) =>
          ipcRenderer.invoke("context:query-context", { query, maxResults }),
        getNodeContext: (nodeId) => ipcRenderer.invoke("context:get-node-context", nodeId),
        listDomains: () => ipcRenderer.invoke("context:list-domains"),
        onIndexProgress: (handler) => {
          ipcRenderer.on("context:index-progress", (_, data) => handler(data));
          return () => ipcRenderer.removeAllListeners("context:index-progress");
        },
        onIndexComplete: (handler) => {
          ipcRenderer.on("context:index-complete", (_, data) => handler(data));
          return () => ipcRenderer.removeAllListeners("context:index-complete");
        }
      }
});
