import { useEffect, useMemo, useRef, useState, useCallback } from "react";
import type { FileEntry, ContextNode } from "./types";
import ContextSidebar from "./ContextSidebar";

const DEFAULT_COMMAND = "";
const DEFAULT_ARGS = "";

type ChatMessage = {
  id: string;
  role: "user" | "assistant" | "system";
  text: string;
  streaming?: boolean;
};

type Favorite = {
  label: string;
  path: string;
};

type ApprovalState = {
  id: number;
  method: "applyPatchApproval" | "execCommandApproval";
  params: Record<string, unknown>;
};

type StatusItem = {
  id: string;
  label: string;
};

function useLocalStorage(key: string, initialValue: string) {
  const [value, setValue] = useState(() => {
    const saved = window.localStorage.getItem(key);
    return saved ?? initialValue;
  });

  useEffect(() => {
    window.localStorage.setItem(key, value);
  }, [key, value]);

  return [value, setValue] as const;
}

function formatDate(ms: number) {
  const date = new Date(ms);
  return date.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "2-digit"
  });
}

function splitArgs(input: string) {
  return input
    .split(" ")
    .map((part) => part.trim())
    .filter(Boolean);
}

export default function App() {
  const [favorites, setFavorites] = useState<Favorite[]>([]);
  const [currentPath, setCurrentPath] = useState<string>("/");
  const [entries, setEntries] = useState<FileEntry[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<FileEntry[]>([]);
  const [recursiveSearch, setRecursiveSearch] = useState(true);
  const [loading, setLoading] = useState(false);
  const [refreshTick, setRefreshTick] = useState(0);

  const [command, setCommand] = useLocalStorage("codex.command", DEFAULT_COMMAND);
  const [args, setArgs] = useLocalStorage("codex.args", DEFAULT_ARGS);
  const [model, setModel] = useLocalStorage("codex.model", "gpt-5.1");

  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [prompt, setPrompt] = useState("");
  const [codexReady, setCodexReady] = useState(false);
  const [statusText, setStatusText] = useState("Idle");
  const [approval, setApproval] = useState<ApprovalState | null>(null);
  const [statusItems, setStatusItems] = useState<StatusItem[]>([]);
  const [showContextSidebar, setShowContextSidebar] = useState(false);
  const [contextNodes, setContextNodes] = useState<ContextNode[]>([]);

  const chatEndRef = useRef<HTMLDivElement | null>(null);
  const idCounter = useRef(0);

  const nextId = (prefix: string) => {
    idCounter.current += 1;
    return `${prefix}-${Date.now()}-${idCounter.current}`;
  };

  const pushStatus = (label: string) => {
    setStatusItems((prev) => [...prev.slice(-200), { id: nextId("status"), label }]);
  };

  useEffect(() => {
    window.codexApi.fs.getFavorites().then((data) => {
      setFavorites(data);
      if (data[0]?.path) {
        setCurrentPath(data[0].path);
      }
    });
  }, []);

  const reloadDirectory = () => {
    setRefreshTick((tick) => tick + 1);
  };

  useEffect(() => {
    let active = true;
    setLoading(true);
    window.codexApi.fs
      .listDirectory(currentPath)
      .then((list) => {
        if (active) {
          setEntries(list);
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [currentPath, refreshTick]);

  useEffect(() => {
    const interval = window.setInterval(() => {
      setRefreshTick((tick) => tick + 1);
    }, 3000);
    const onFocus = () => reloadDirectory();
    window.addEventListener("focus", onFocus);
    return () => {
      window.clearInterval(interval);
      window.removeEventListener("focus", onFocus);
    };
  }, []);

  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }

    const handle = window.setTimeout(() => {
      window.codexApi.fs
        .searchDirectory({
          root: currentPath,
          query: searchQuery,
          recursive: recursiveSearch
        })
        .then((results) => setSearchResults(results));
    }, 250);

    return () => window.clearTimeout(handle);
  }, [searchQuery, currentPath, recursiveSearch]);

  useEffect(() => {
    const unsubEvent = window.codexApi.codex.onEvent((event) => {
      const msg = event.msg;
      switch (msg.type) {
        case "agent_message": {
          const messageText = String(msg.message ?? "");
          setMessages((prev) => {
            const last = prev[prev.length - 1];
            if (last && last.role === "assistant" && last.streaming) {
              return prev.map((item, idx) =>
                idx === prev.length - 1
                  ? { ...item, text: messageText, streaming: false }
                  : item
              );
            }
            return [
              ...prev,
              { id: nextId("assistant"), role: "assistant", text: messageText }
            ];
          });
          break;
        }
        case "agent_message_delta": {
          const delta = String(msg.delta ?? "");
          setMessages((prev) => {
            const last = prev[prev.length - 1];
            if (last && last.role === "assistant" && last.streaming) {
              return prev.map((item, idx) =>
                idx === prev.length - 1
                  ? { ...item, text: item.text + delta }
                  : item
              );
            }
            return [
              ...prev,
              { id: nextId("assistant"), role: "assistant", text: delta, streaming: true }
            ];
          });
          break;
        }
        case "task_complete": {
          setMessages((prev) =>
            prev.map((item) =>
              item.streaming ? { ...item, streaming: false } : item
            )
          );
          setStatusText("Ready");
          pushStatus("task_complete");
          break;
        }
        case "error": {
          const messageText = String(msg.message ?? "Unknown error");
          setMessages((prev) => [
            ...prev,
            { id: nextId("error"), role: "system", text: messageText }
          ]);
          pushStatus(`error: ${messageText}`);
          break;
        }
        case "exec_command_output_delta": {
          let text = "";
          if (typeof msg.chunk === "string") {
            try {
              text = atob(msg.chunk);
            } catch {
              text = msg.chunk;
            }
          } else {
            text = String(msg.chunk ?? "");
          }
          setMessages((prev) => [
            ...prev,
            { id: nextId("exec"), role: "system", text }
          ]);
          pushStatus("exec_command_output_delta");
          break;
        }
        default:
          // Log unhandled event types for debugging
          // Don't create UI messages for unknown event types
          pushStatus(`unhandled_event: ${msg.type}`);
          break;
      }
    });

    const unsubApproval = window.codexApi.codex.onApproval((payload) => {
      setApproval(payload);
    });

    const unsubStatus = window.codexApi.codex.onStatus((payload) => {
      const label = payload.text ? `${payload.type}: ${payload.text}` : payload.type;
      pushStatus(label);
      if (payload.type === "exit") {
        setCodexReady(false);
        setStatusText("Codex stopped");
      }
      if (payload.type === "stderr") {
        setStatusText("Codex error output");
      }
    });

    // Listen for codex ready event (from context operations that start the server)
    const unsubReady = window.codexApi.codex.onReady(() => {
      setCodexReady(true);
      setStatusText("Ready");
    });

    return () => {
      unsubEvent();
      unsubApproval();
      unsubStatus();
      unsubReady();
    };
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  // Keyboard shortcut for context sidebar (Cmd+K / Ctrl+K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        setShowContextSidebar((prev) => !prev);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // Handler for when context nodes are selected
  const handleContextNodesSelected = useCallback((nodes: ContextNode[]) => {
    setContextNodes(nodes);

    // Format context for prompt
    const contextText = nodes.map((node) => {
      let text = `## ${node.name} (${node.node_type})\n`;
      text += `${node.summary}\n`;
      if (node.path) text += `Path: ${node.path}\n`;
      if (node.keywords.length > 0) text += `Keywords: ${node.keywords.join(", ")}\n`;
      return text;
    }).join("\n---\n");

    // Add system message about context
    setMessages((prev) => [
      ...prev,
      {
        id: nextId("system"),
        role: "system",
        text: `Added ${nodes.length} context nodes:\n${contextText}`
      }
    ]);

    // Optionally close sidebar
    setShowContextSidebar(false);
  }, []);

  const activeEntries = searchQuery.trim() ? searchResults : entries;
  const breadcrumb = useMemo(() => currentPath.split("/").filter(Boolean), [currentPath]);

  async function startCodexIfNeeded() {
    if (codexReady) {
      return true;
    }
    const trimmedCommand = command.trim();
    const trimmedArgs = args.trim();
    const legacyArgs = trimmedArgs.includes("codex-cli/bin/codex.js");
    if (trimmedCommand === "node" && legacyArgs) {
      setCommand("");
      setArgs("");
      pushStatus("cleared legacy CLI command; using MCP binary");
    }
    setStatusText("Starting Codex...");
    try {
      const options: {
        model: string;
        cwd: string;
        command?: string;
        args?: string[];
      } = {
        model,
        cwd: currentPath
      };
      if (trimmedCommand && !(trimmedCommand === "node" && legacyArgs)) {
        options.command = trimmedCommand;
        options.args = splitArgs(trimmedArgs);
      }
      await window.codexApi.codex.start(options);
      setCodexReady(true);
      setStatusText("Ready");
      return true;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setMessages((prev) => [
        ...prev,
        { id: nextId("start-error"), role: "system", text: message }
      ]);
      setStatusText("Codex failed to start");
      pushStatus(`start_failed: ${message}`);
      return false;
    }
  }

  async function handleSend() {
    const userText = prompt.trim(); // Original text to display in chat
    let textToSend = userText; // Text with context to send to AI
    if (!userText) {
      return;
    }
    setPrompt("");

    // Inject manually selected context if any
    if (contextNodes.length > 0) {
      const contextText = contextNodes.map((node) => {
        let nodeText = `## ${node.name} (${node.node_type})\n`;
        nodeText += `${node.summary}\n`;
        if (node.path) nodeText += `Path: ${node.path}\n`;
        if (node.keywords.length > 0) nodeText += `Keywords: ${node.keywords.join(", ")}\n`;
        return nodeText;
      }).join("\n---\n");

      textToSend = `<context>\n${contextText}\n</context>\n\n${userText}`;
      setContextNodes([]); // Clear context after using
    } else {
      // Auto-query context based on user's message (if no manual selection)
      try {
        pushStatus(`Querying context for: "${userText.slice(0, 50)}..."`);
        const result = await window.codexApi.context.queryContext(userText, 10);
        pushStatus(`Context query returned ${result.nodes?.length ?? 0} nodes`);

        if (result.nodes && result.nodes.length > 0) {
          const autoContext = result.nodes.map((node: ContextNode) => {
            let nodeText = `## ${node.name} (${node.node_type})\n`;
            nodeText += `${node.summary}\n`;
            if (node.path) nodeText += `Path: ${node.path}\n`;
            return nodeText;
          }).join("\n---\n");

          textToSend = `<relevant_context>\n${autoContext}\n</relevant_context>\n\n${userText}`;
          pushStatus(`Auto-injected ${result.nodes.length} context nodes`);
        } else {
          pushStatus("No matching context nodes found");
        }
      } catch (err) {
        // Context query failed - log the error
        pushStatus(`Context query error: ${err}`);
        console.error("Context query failed:", err);
      }
    }
    
    // Handle slash commands
    if (userText.startsWith("/")) {
      const command = userText.split(/\s+/)[0];
      
      // Handle /init command
      if (command === "/init") {
        const started = await startCodexIfNeeded();
        if (!started) {
          return;
        }
        
        // Check if AGENTS.md already exists
        const agentsExists = await window.codexApi.fs.checkAgentsMd(currentPath);
        if (agentsExists) {
          setMessages((prev) => [
            ...prev,
            { id: nextId("info"), role: "system", text: "AGENTS.md already exists here. Skipping /init to avoid overwriting it." }
          ]);
          return;
        }
        
        // Send the special /init prompt instead of the literal "/init" text
        // This matches the prompt from codex-rs/tui/prompt_for_init_command.md
        const initPrompt = `Generate a file named AGENTS.md that serves as a contributor guide for this repository.
Your goal is to produce a clear, concise, and well-structured document with descriptive headings and actionable explanations for each section.
Follow the outline below, but adapt as needed — add sections if relevant, and omit those that do not apply to this project.

Document Requirements

- Title the document "Repository Guidelines".
- Use Markdown headings (#, ##, etc.) for structure.
- Keep the document concise. 200-400 words is optimal.
- Keep explanations short, direct, and specific to this repository.
- Provide examples where helpful (commands, directory paths, naming patterns).
- Maintain a professional, instructional tone.

Recommended Sections

Project Structure & Module Organization

- Outline the project structure, including where the source code, tests, and assets are located.

Build, Test, and Development Commands

- List key commands for building, testing, and running locally (e.g., npm test, make build).
- Briefly explain what each command does.

Coding Style & Naming Conventions

- Specify indentation rules, language-specific style preferences, and naming patterns.
- Include any formatting or linting tools used.

Testing Guidelines

- Identify testing frameworks and coverage requirements.
- State test naming conventions and how to run tests.

Commit & Pull Request Guidelines

- Summarize commit message conventions found in the project's Git history.
- Outline pull request requirements (descriptions, linked issues, screenshots, etc.).

(Optional) Add other sections if relevant, such as Security & Configuration Tips, Architecture Overview, or Agent-Specific Instructions.`;

        setMessages((prev) => [
          ...prev,
          { id: nextId("user"), role: "user", text: "/init" }
        ]);
        setStatusText("Thinking...");
        pushStatus("send_user_turn");
        try {
          await window.codexApi.codex.send({ text: initPrompt, cwd: currentPath });
        } catch (error) {
          const message = error instanceof Error ? error.message : String(error);
          setMessages((prev) => [
            ...prev,
            { id: nextId("send-error"), role: "system", text: message }
          ]);
          setStatusText("Send failed");
          pushStatus(`send_failed: ${message}`);
        }
        return;
      }
      
      // For other slash commands, show a message that they're not yet implemented
      if (command !== "/init") {
        setMessages((prev) => [
          ...prev,
          { id: nextId("info"), role: "system", text: `Slash command '${command}' is not yet implemented in this interface.` }
        ]);
        return;
      }
    }
    
    const started = await startCodexIfNeeded();
    if (!started) {
      return;
    }
    setMessages((prev) => [
      ...prev,
      { id: nextId("user"), role: "user", text: userText }
    ]);
    setStatusText("Thinking...");
    pushStatus("send_user_turn");
    try {
      await window.codexApi.codex.send({ text: textToSend, cwd: currentPath });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setMessages((prev) => [
        ...prev,
        { id: nextId("send-error"), role: "system", text: message }
      ]);
      setStatusText("Send failed");
      pushStatus(`send_failed: ${message}`);
    }
  }

  async function handleOpen(entry: FileEntry) {
    if (entry.isDir) {
      setCurrentPath(entry.path);
    } else {
      await window.codexApi.fs.openPath(entry.path);
    }
  }

  async function handleOpenWith(entry: FileEntry) {
    await window.codexApi.fs.openWith(entry.path);
  }

  async function handleReveal(entry: FileEntry) {
    await window.codexApi.fs.revealInFinder(entry.path);
  }

  async function handlePickFolder() {
    const picked = await window.codexApi.fs.pickFolder();
    if (picked) {
      setCurrentPath(picked);
    }
  }

  async function handleApproval(decision: "allow" | "deny") {
    if (!approval) {
      return;
    }
    await window.codexApi.codex.approve({ id: approval.id, decision });
    setApproval(null);
  }

  return (
    <div className="app" style={{ display: "flex", flexDirection: "row" }}>
      {showContextSidebar && (
        <ContextSidebar
          onSelectNodes={handleContextNodesSelected}
          onClose={() => setShowContextSidebar(false)}
        />
      )}
      <div style={{ flex: 1, display: "flex", flexDirection: "column", height: "100vh", overflow: "hidden" }}>
      <div className="app__background" />
      <header className="app__topbar">
        <div>
          <h1>Codex Desktop</h1>
          <p>Electron finder with Codex in the flow</p>
        </div>
        <div className="status-pill">
          <span className={`status-dot ${codexReady ? "ready" : "idle"}`} />
          {statusText}
        </div>
      </header>

      <main className="app__main">
        <section className="finder">
          <div className="finder__sidebar">
            <div className="sidebar__header">
              <span>Places</span>
              <button className="ghost" onClick={handlePickFolder}>
                Pick folder
              </button>
            </div>
            <div className="sidebar__list">
              {favorites.map((fav) => (
                <button
                  key={fav.path}
                  className={fav.path === currentPath ? "sidebar__item active" : "sidebar__item"}
                  onClick={() => setCurrentPath(fav.path)}
                >
                  <span className="sidebar__dot" />
                  {fav.label}
                </button>
              ))}
            </div>
          </div>

          <div className="finder__content">
            <div className="finder__toolbar">
              <div className="breadcrumb">
                <button className="crumb" onClick={() => setCurrentPath("/")}>/
                </button>
                {breadcrumb.map((crumb, idx) => {
                  const crumbPath = "/" + breadcrumb.slice(0, idx + 1).join("/");
                  return (
                    <button
                      key={crumbPath}
                      className="crumb"
                      onClick={() => setCurrentPath(crumbPath)}
                    >
                      {crumb}
                    </button>
                  );
                })}
              </div>
              <div className="search">
                <input
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                  placeholder="Search in this folder"
                />
                <button className="ghost" onClick={reloadDirectory}>
                  Refresh
                </button>
                <label className="toggle">
                  <input
                    type="checkbox"
                    checked={recursiveSearch}
                    onChange={(event) => setRecursiveSearch(event.target.checked)}
                  />
                  <span>Recursive</span>
                </label>
              </div>
            </div>

            <div className="finder__header">
              <span>Name</span>
              <span>Modified</span>
              <span>Actions</span>
            </div>

            <div className={loading ? "finder__list loading" : "finder__list"}>
              {activeEntries.map((entry) => (
                <div className="finder__row" key={entry.path}>
                  <button
                    className={entry.isDir ? "entry entry--dir" : "entry"}
                    onDoubleClick={() => handleOpen(entry)}
                    onClick={() => entry.isDir && setCurrentPath(entry.path)}
                  >
                    <span className={entry.isDir ? "icon folder" : "icon file"} />
                    <span className="entry__name">{entry.name}</span>
                  </button>
                  <span className="muted">{formatDate(entry.mtimeMs)}</span>
                  <div className="row__actions">
                    <button onClick={() => handleOpen(entry)}>Open</button>
                    <button onClick={() => handleReveal(entry)}>Reveal</button>
                    <button onClick={() => handleOpenWith(entry)}>Open with...</button>
                  </div>
                </div>
              ))}
              {!loading && activeEntries.length === 0 && (
                <div className="finder__empty">No matching files found</div>
              )}
            </div>
          </div>
        </section>

        <section className="chat">
          <div className="chat__header">
            <div>
              <h2>Codex prompt</h2>
              <span>Workspace: {currentPath}</span>
            </div>
            <div className="chat__settings">
              <label>
                Model
                <input value={model} onChange={(event) => setModel(event.target.value)} />
              </label>
              <label>
                Command
                <input value={command} onChange={(event) => setCommand(event.target.value)} />
              </label>
              <label>
                Args
                <input value={args} onChange={(event) => setArgs(event.target.value)} />
              </label>
            </div>
          </div>

          <div className="chat__messages">
            {messages
              .filter((message) => message.text && message.text.trim().length > 0)
              .map((message) => (
                <div key={message.id} className={`message ${message.role}`}>
                  <div className="message__role">{message.role}</div>
                  <div className="message__text">{message.text}</div>
                </div>
              ))}
            <div ref={chatEndRef} />
          </div>

          <div className="chat__debug">
            <div className="chat__debug-header">Debug stream</div>
            <div className="chat__debug-body">
              {statusItems.map((item) => (
                <div key={item.id} className="debug-line">
                  {item.label}
                </div>
              ))}
            </div>
          </div>

          <div className="chat__composer">
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === "Enter" && !event.shiftKey) {
                  event.preventDefault();
                  handleSend();
                }
              }}
              placeholder="Ask Codex to scan, modify, test, or explain..."
              rows={3}
            />
            <div className="composer__actions">
              <button className="primary" onClick={handleSend}>
                Send
              </button>
              <span className="hint">Shift + Enter for newline</span>
            </div>
          </div>
        </section>
      </main>

      {approval && (
        <div className="modal">
          <div className="modal__card">
            <h3>Approval required</h3>
            <p>{approval.method === "execCommandApproval" ? "Run command?" : "Apply patch?"}</p>
            <pre>{JSON.stringify(approval.params, null, 2)}</pre>
            <div className="modal__actions">
              <button onClick={() => handleApproval("deny")}>Deny</button>
              <button className="primary" onClick={() => handleApproval("allow")}>
                Allow
              </button>
            </div>
          </div>
        </div>
      )}
      </div>
    </div>
  );
}
