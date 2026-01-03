const { app, BrowserWindow, ipcMain, dialog, shell } = require("electron");
const path = require("path");
const { spawn } = require("child_process");
const fs = require("fs/promises");
const os = require("os");

const DEFAULT_SERVER = {
  command: "",
  args: [],
  cwd: ""
};

const SKIP_DIRS = new Set([
  "node_modules",
  ".git",
  "dist",
  "build",
  "target",
  ".cache",
  "Library",
  ".docker",
  ".docker-desktop",
  ".docker-desktop-data"
]);

function shouldHideEntry(name) {
  if (!name) {
    return true;
  }
  if (name.startsWith(".")) {
    return true;
  }
  return false;
}

let mainWindow;
let appServerClient;
const isDev = Boolean(process.env.VITE_DEV_SERVER_URL);

async function findRepoRoot(startDir) {
  let current = startDir;
  while (current && current !== path.dirname(current)) {
    const candidate = path.join(current, "codex-rs", "Cargo.toml");
    if (await fileExists(candidate)) {
      return current;
    }
    current = path.dirname(current);
  }
  return null;
}

async function resolveRepoRoot() {
  if (process.env.CODEX_REPO_ROOT) {
    return process.env.CODEX_REPO_ROOT;
  }
  const appPath = app.getAppPath();
  const repoRoot = await findRepoRoot(appPath);
  return repoRoot ?? process.cwd();
}

async function resolveCargoRoot(repoRoot) {
  const codexRs = path.join(repoRoot, "codex-rs");
  if (await fileExists(path.join(codexRs, "Cargo.toml"))) {
    return codexRs;
  }
  return repoRoot;
}

class AppServerClient {
  constructor({ command, args, cwd }) {
    this.command = command;
    this.args = args;
    this.cwd = cwd;
    this.child = null;
    this.buffer = "";
    this.nextId = 1;
    this.pending = new Map();
    this.pendingApprovals = new Map();
    this.threadId = null;
    this.initialized = false;
  }

  start() {
    if (this.child) {
      return;
    }
    this.emitStatus({ type: "server", text: "Starting Codex app-server..." });
    this.child = spawn(this.command, this.args, {
      cwd: this.cwd,
      stdio: ["pipe", "pipe", "pipe"],
      env: { ...process.env }
    });

    this.child.stdout.setEncoding("utf8");
    this.child.stdout.on("data", (chunk) => this.onData(chunk));
    this.child.stderr.setEncoding("utf8");
    this.child.stderr.on("data", (chunk) => {
      this.emitStatus({ type: "stderr", text: chunk });
    });
    this.child.on("exit", (code) => {
      this.emitStatus({ type: "exit", code });
      this.child = null;
    });
  }

  stop() {
    if (!this.child) {
      return;
    }
    this.child.kill();
    this.child = null;
  }

  emitStatus(payload) {
    if (mainWindow) {
      mainWindow.webContents.send("codex:status", payload);
    }
  }

  onData(chunk) {
    this.buffer += chunk;
    let newlineIndex = this.buffer.indexOf("\n");
    while (newlineIndex >= 0) {
      const line = this.buffer.slice(0, newlineIndex).trim();
      this.buffer = this.buffer.slice(newlineIndex + 1);
      if (line) {
        this.handleLine(line);
      }
      newlineIndex = this.buffer.indexOf("\n");
    }
  }

  handleLine(line) {
    let msg;
    try {
      msg = JSON.parse(line);
    } catch (err) {
      this.emitStatus({ type: "parse_error", text: line });
      return;
    }

    // Handle JSON-RPC responses (requests with id)
    if (msg.id !== undefined && msg.result !== undefined) {
      const pending = this.pending.get(msg.id);
      this.emitStatus({ type: "response", id: msg.id });
      if (pending) {
        this.pending.delete(msg.id);
        pending.resolve(msg.result);
      }
      return;
    }

    if (msg.id !== undefined && msg.error) {
      const pending = this.pending.get(msg.id);
      this.emitStatus({ type: "response_error", id: msg.id, error: msg.error });
      if (pending) {
        this.pending.delete(msg.id);
        pending.reject(new Error(msg.error.message || "Request failed"));
      }
      return;
    }

    // Handle server-initiated requests (approvals) - these have an id but are requests from server
    if (msg.id !== undefined && msg.method && msg.method.includes("requestApproval")) {
      const approvalId = msg.id;
      this.pendingApprovals.set(approvalId, msg);
      
      if (msg.method === "item/commandExecution/requestApproval") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:approval", {
            id: approvalId,
            method: "execCommandApproval",
            params: {
              itemId: msg.params?.itemId,
              threadId: msg.params?.threadId,
              turnId: msg.params?.turnId,
              command: msg.params?.parsedCmd || msg.params?.command,
              cwd: msg.params?.cwd,
              reason: msg.params?.reason,
              risk: msg.params?.risk
            }
          });
        }
      } else if (msg.method === "item/fileChange/requestApproval") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:approval", {
            id: approvalId,
            method: "applyPatchApproval",
            params: {
              itemId: msg.params?.itemId,
              threadId: msg.params?.threadId,
              turnId: msg.params?.turnId,
              changes: msg.params?.changes,
              reason: msg.params?.reason
            }
          });
        }
      }
      return;
    }

    // Handle notifications (no id) - app-server events
    if (!msg.id && msg.method) {
      // Convert app-server events to codex/event format for compatibility
      if (msg.method === "item/agentMessage/delta") {
        if (mainWindow && msg.params) {
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "agent_message_delta",
              delta: msg.params.delta || ""
            }
          });
        }
        return;
      }

      if (msg.method === "item/started") {
        if (mainWindow && msg.params && msg.params.item) {
          const item = msg.params.item;
          // Handle agent messages that start immediately
          if (item.type === "agentMessage" && item.text) {
            mainWindow.webContents.send("codex:event", {
              msg: {
                type: "agent_message",
                message: item.text
              }
            });
          }
          // For other item types (reasoning, fileChange, commandExecution, etc.),
          // we don't need to create UI messages - they're handled by their specific events
          // (like item/commandExecution/outputDelta, item/agentMessage/delta, etc.)
          // Just log for debugging
          if (item.type !== "agentMessage") {
            this.emitStatus({ 
              type: "item_started", 
              text: `Item started: ${item.type} (id: ${item.id || "unknown"})` 
            });
          }
        }
        return;
      }

      if (msg.method === "item/agentMessage/completed" || msg.method === "item/completed") {
        if (mainWindow && msg.params && msg.params.item) {
          const item = msg.params.item;
          if (item.type === "agentMessage" && item.text) {
            mainWindow.webContents.send("codex:event", {
              msg: {
                type: "agent_message",
                message: item.text
              }
            });
          }
          // Log completion of other item types for debugging
          if (item.type !== "agentMessage") {
            this.emitStatus({ 
              type: "item_completed", 
              text: `Item completed: ${item.type} (id: ${item.id || "unknown"})` 
            });
          }
        }
        return;
      }

      if (msg.method === "turn/completed") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "task_complete"
            }
          });
        }
        return;
      }

      if (msg.method === "item/commandExecution/outputDelta") {
        if (mainWindow && msg.params) {
          const chunk = msg.params.chunk || "";
          // App-server sends output as string, encode to base64 for compatibility
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "exec_command_output_delta",
              chunk: Buffer.from(chunk, "utf8").toString("base64")
            }
          });
        }
        return;
      }

      // Handle error events
      if (msg.method === "error") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "error",
              message: msg.params?.error?.message || "Unknown error"
            }
          });
        }
        return;
      }

      // Handle account/auth notifications
      if (msg.method === "account/login/completed") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "account_login_completed",
              ...msg.params
            }
          });
        }
        return;
      }

      if (msg.method === "account/updated") {
        if (mainWindow) {
          mainWindow.webContents.send("codex:event", {
            msg: {
              type: "account_updated",
              ...msg.params
            }
          });
        }
        return;
      }


      // Log unhandled notifications for debugging (don't forward as UI events)
      // Many app-server notifications are internal and shouldn't create UI messages
      this.emitStatus({ 
        type: "unhandled_notification", 
        text: `Unhandled notification: ${msg.method}` 
      });
      return;
    }
  }

  request(method, params) {
    if (!this.child) {
      throw new Error("Codex server not running");
    }
    const id = this.nextId++;
    this.emitStatus({ type: "request", id, text: method });
    // App-server uses JSON-RPC 2.0 but omits the jsonrpc field
    const payload = {
      id,
      method,
      params: params || {}
    };
    const line = JSON.stringify(payload);
    this.child.stdin.write(line + "\n");
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
    });
  }

  notify(method, params) {
    if (!this.child) {
      throw new Error("Codex server not running");
    }
    // App-server uses JSON-RPC 2.0 but omits the jsonrpc field
    const payload = {
      method,
      params: params || {}
    };
    this.child.stdin.write(JSON.stringify(payload) + "\n");
  }

  async initialize(clientInfo) {
    if (this.initialized) {
      return;
    }
    const result = await this.request("initialize", {
      clientInfo: clientInfo ?? {
        name: "codex-electron",
        title: "Codex Desktop",
        version: "0.1.0"
      }
    });
    // Send initialized notification (required by app-server)
    this.notify("initialized", {});
    this.initialized = true;
    return result;
  }

  respondToApproval(id, decision) {
    if (!this.child) {
      throw new Error("Codex server not running");
    }
    const approval = this.pendingApprovals.get(id);
    if (!approval) {
      this.emitStatus({ type: "approval_error", text: `Approval ${id} not found` });
      return;
    }
    this.pendingApprovals.delete(id);
    this.emitStatus({ type: "approval_response", id, decision });
    
    // App-server approval response format - respond to the server's request ID
    const payload = {
      id: approval.id || id,
      result: {
        decision: decision === "allow" ? "accept" : "decline",
        acceptSettings: decision === "allow" ? { forSession: false } : undefined
      }
    };
    this.child.stdin.write(JSON.stringify(payload) + "\n");
  }
}

async function fileExists(targetPath) {
  try {
    await fs.access(targetPath);
    return true;
  } catch {
    return false;
  }
}

async function resolveCargoEnv() {
  const home = os.homedir();
  const cargoBin = path.join(home, ".cargo", "bin");
  const pathSep = process.platform === "win32" ? ";" : ":";
  const parts = (process.env.PATH || "").split(pathSep).filter(Boolean);
  if (!parts.includes(cargoBin)) {
    parts.unshift(cargoBin);
  }
  return { ...process.env, PATH: parts.join(pathSep) };
}

async function findOnPath(binaryName, envPath) {
  const pathSep = process.platform === "win32" ? ";" : ":";
  const entries = envPath.split(pathSep).filter(Boolean);
  for (const entry of entries) {
    const fullPath = path.join(entry, binaryName);
    if (await fileExists(fullPath)) {
      return fullPath;
    }
  }
  return null;
}

async function buildAppServerBinary(repoRoot) {
  const cargoRoot = await resolveCargoRoot(repoRoot);
  const cargoEnv = await resolveCargoEnv();
  const cargoPath = await findOnPath(
    process.platform === "win32" ? "cargo.exe" : "cargo",
    cargoEnv.PATH || ""
  );
  if (!cargoPath) {
    throw new Error("cargo not found in PATH; install Rust or set PATH for Electron.");
  }
  await new Promise((resolve, reject) => {
    const build = spawn("cargo", ["build", "-p", "codex-app-server"], {
      cwd: cargoRoot,
      stdio: ["ignore", "pipe", "pipe"],
      env: cargoEnv
    });

    build.stdout.setEncoding("utf8");
    build.stdout.on("data", (chunk) => {
      if (mainWindow) {
        mainWindow.webContents.send("codex:status", { type: "build", text: chunk });
      }
    });
    build.stderr.setEncoding("utf8");
    build.stderr.on("data", (chunk) => {
      if (mainWindow) {
        mainWindow.webContents.send("codex:status", { type: "build", text: chunk });
      }
    });
    build.on("error", reject);
    build.on("exit", (code) => {
      if (code === 0) {
        resolve();
      } else {
        reject(new Error(`cargo build failed with code ${code}`));
      }
    });
  });
}

async function resolveAppServerCommand() {
  const repoRoot = await resolveRepoRoot();
  const cargoRoot = await resolveCargoRoot(repoRoot);
  const binName = process.platform === "win32" ? "codex-app-server.exe" : "codex-app-server";
  const binPath = path.join(cargoRoot, "target", "debug", binName);

  if (!(await fileExists(binPath))) {
    await buildAppServerBinary(repoRoot);
  }

  return { command: binPath, args: [], cwd: repoRoot };
}

function createWindow() {
  mainWindow = new BrowserWindow({
    width: 1320,
    height: 860,
    minWidth: 980,
    minHeight: 640,
    backgroundColor: "#0e1116",
    titleBarStyle: "hiddenInset",
    webPreferences: {
      preload: path.join(__dirname, "preload.js"),
      contextIsolation: true,
      nodeIntegration: false
    }
  });

  if (process.env.VITE_DEV_SERVER_URL) {
    mainWindow.loadURL(process.env.VITE_DEV_SERVER_URL);
    mainWindow.webContents.openDevTools({ mode: "detach" });
  } else {
    const indexHtml = path.join(__dirname, "..", "dist", "renderer", "index.html");
    mainWindow.loadFile(indexHtml);
  }
}

async function listDirectory(dirPath) {
  const entries = await fs.readdir(dirPath, { withFileTypes: true });
  const detailed = await Promise.all(
    entries.map(async (entry) => {
      if (shouldHideEntry(entry.name) || SKIP_DIRS.has(entry.name)) {
        return null;
      }
      const fullPath = path.join(dirPath, entry.name);
      try {
        const stat = await fs.stat(fullPath);
        return {
          name: entry.name,
          path: fullPath,
          isDir: entry.isDirectory(),
          size: stat.size,
          mtimeMs: stat.mtimeMs
        };
      } catch {
        return null;
      }
    })
  );

  return detailed.filter(Boolean).sort((a, b) => {
    if (a.isDir !== b.isDir) {
      return a.isDir ? -1 : 1;
    }
    return a.name.localeCompare(b.name);
  });
}

async function searchDirectory(root, query, recursive, maxResults) {
  const results = [];
  const q = query.toLowerCase();

  async function walk(dir, depth) {
    if (results.length >= maxResults) {
      return;
    }
    let entries;
    try {
      entries = await fs.readdir(dir, { withFileTypes: true });
    } catch (err) {
      return;
    }

    for (const entry of entries) {
      if (results.length >= maxResults) {
        break;
      }
      if (shouldHideEntry(entry.name) || SKIP_DIRS.has(entry.name)) {
        continue;
      }
      const fullPath = path.join(dir, entry.name);
      const nameMatch = entry.name.toLowerCase().includes(q);
      if (nameMatch) {
        try {
          const stat = await fs.stat(fullPath);
          results.push({
            name: entry.name,
            path: fullPath,
            isDir: entry.isDirectory(),
            size: stat.size,
            mtimeMs: stat.mtimeMs
          });
        } catch (err) {
          continue;
        }
      }

      if (recursive && entry.isDirectory() && depth < 6) {
        await walk(fullPath, depth + 1);
      }
    }
  }

  await walk(root, 0);
  return results;
}

function favoritePaths() {
  const home = os.homedir();
  const favorites = [
    { label: "Home", path: home },
    { label: "Desktop", path: path.join(home, "Desktop") },
    { label: "Documents", path: path.join(home, "Documents") },
    { label: "Downloads", path: path.join(home, "Downloads") },
    { label: "Applications", path: "/Applications" },
    { label: "Root", path: "/" },
    { label: "Volumes", path: "/Volumes" }
  ];
  return favorites.filter((fav) => fav.path && fav.path !== "" );
}

function setupIpc() {
  ipcMain.handle("fs:list", async (_, dirPath) => listDirectory(dirPath));
  ipcMain.handle("fs:search", async (_, payload) => {
    const { root, query, recursive } = payload;
    return searchDirectory(root, query, recursive, 500);
  });
  ipcMain.handle("fs:favorites", async () => favoritePaths());
  ipcMain.handle("fs:pick-folder", async () => {
    const result = await dialog.showOpenDialog({ properties: ["openDirectory"] });
    return result.canceled ? null : result.filePaths[0];
  });
  ipcMain.handle("fs:open", async (_, targetPath) => shell.openPath(targetPath));
  ipcMain.handle("fs:reveal", async (_, targetPath) => {
    shell.showItemInFolder(targetPath);
  });
  ipcMain.handle("fs:open-with", async (_, targetPath) => {
    const result = await dialog.showOpenDialog({
      properties: ["openFile"],
      defaultPath: "/Applications",
      filters: [{ name: "Applications", extensions: ["app"] }]
    });
    if (result.canceled || result.filePaths.length === 0) {
      return { canceled: true };
    }
    const appPath = result.filePaths[0];
    await new Promise((resolve, reject) => {
      const child = spawn("open", ["-a", appPath, targetPath]);
      child.on("exit", resolve);
      child.on("error", reject);
    });
    return { canceled: false, appPath };
  });

  ipcMain.handle("codex:start", async (_, options) => {
    if (appServerClient && appServerClient.child) {
      return { conversationId: appServerClient.threadId };
    }

    if (mainWindow) {
      mainWindow.webContents.send("codex:status", {
        type: "server",
        text: "Preparing Codex app-server..."
      });
    }
    let serverConfig = null;
    const repoRoot = await resolveRepoRoot();
    if (options?.command) {
      const candidateCommand = options.command;
      const candidateArgs = options?.args ?? [];
      const isNode = ["node", "node.exe"].includes(path.basename(candidateCommand));
      const nodeTarget = candidateArgs[0];
      if (isNode && nodeTarget && !(await fileExists(nodeTarget))) {
        if (mainWindow) {
          mainWindow.webContents.send("codex:status", {
            type: "server",
            text: `Ignoring invalid Node target: ${nodeTarget}`
          });
        }
      } else if (!(await fileExists(candidateCommand)) && path.isAbsolute(candidateCommand)) {
        if (mainWindow) {
          mainWindow.webContents.send("codex:status", {
            type: "server",
            text: `Ignoring missing command: ${candidateCommand}`
          });
        }
      } else {
        serverConfig = {
          command: candidateCommand,
          args: candidateArgs,
          cwd: repoRoot
        };
      }
    }

    if (!serverConfig) {
      serverConfig = await resolveAppServerCommand();
    }

    appServerClient = new AppServerClient(serverConfig);
    appServerClient.start();

    await appServerClient.initialize();

    // Use thread/start instead of newConversation
    const threadParams = {};
    if (options?.model) {
      threadParams.model = options.model;
    }
    if (options?.cwd) {
      threadParams.cwd = options.cwd;
    }
    if (options?.approvalPolicy) {
      threadParams.approvalPolicy = options.approvalPolicy;
    }
    if (options?.sandbox) {
      threadParams.sandbox = options.sandbox;
    }

    const result = await appServerClient.request("thread/start", threadParams);
    appServerClient.threadId = result.thread?.id;
    if (mainWindow) {
      mainWindow.webContents.send("codex:status", {
        type: "server",
        text: `Thread started: ${result.thread?.id}`
      });
    }
    return { conversationId: result.thread?.id };
  });

  ipcMain.handle("codex:send", async (_, payload) => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    if (!appServerClient.threadId) {
      throw new Error("No active thread");
    }
    const { text, cwd } = payload;
    
    // Use turn/start instead of sendUserMessage
    const turnParams = {
      threadId: appServerClient.threadId,
      input: [{ type: "text", text }]
    };
    if (cwd) {
      turnParams.cwd = cwd;
    }
    
    return appServerClient.request("turn/start", turnParams);
  });

  ipcMain.handle("codex:approve", async (_, payload) => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    appServerClient.respondToApproval(payload.id, payload.decision);
    return { ok: true };
  });

  ipcMain.handle("codex:stop", async () => {
    if (appServerClient) {
      appServerClient.stop();
      appServerClient = null;
    }
    return { ok: true };
  });

  ipcMain.handle("codex:account:read", async (_, params) => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    return appServerClient.request("account/read", params || {});
  });

  ipcMain.handle("codex:account:login:start", async (_, params) => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    return appServerClient.request("account/login/start", params);
  });

  ipcMain.handle("codex:account:login:cancel", async (_, params) => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    return appServerClient.request("account/login/cancel", params);
  });

  ipcMain.handle("codex:account:logout", async () => {
    if (!appServerClient) {
      throw new Error("Codex not started");
    }
    return appServerClient.request("account/logout", {});
  });

  ipcMain.handle("fs:checkAgentsMd", async (_, dirPath) => {
    const agentsPath = path.join(dirPath, "AGENTS.md");
    return await fileExists(agentsPath);
  });
}

app.whenReady().then(() => {
  const csp = isDev
    ? [
        "default-src 'self'",
        "script-src 'self' 'unsafe-eval' 'unsafe-inline' http://127.0.0.1:5173",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data:",
        "connect-src 'self' http://127.0.0.1:5173 ws://127.0.0.1:5173",
        "font-src 'self' data:"
      ].join("; ")
    : [
        "default-src 'self'",
        "script-src 'self'",
        "style-src 'self' 'unsafe-inline'",
        "img-src 'self' data:",
        "connect-src 'self'",
        "font-src 'self' data:"
      ].join("; ");

  const { session } = require("electron");
  session.defaultSession.webRequest.onHeadersReceived((details, callback) => {
    const headers = details.responseHeaders || {};
    headers["Content-Security-Policy"] = [csp];
    callback({ responseHeaders: headers });
  });

  createWindow();
  setupIpc();

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on("window-all-closed", () => {
  if (process.platform !== "darwin") {
    app.quit();
  }
});
