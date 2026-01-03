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
        }
      }
});
