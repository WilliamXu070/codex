import { useState, useEffect, useCallback } from "react";
import type {
  ContextNode,
  IndexStatus,
  IndexCompleteNotification,
} from "./types";

interface ContextSidebarProps {
  onSelectNodes: (nodes: ContextNode[]) => void;
  onClose: () => void;
}

export default function ContextSidebar({
  onSelectNodes,
  onClose,
}: ContextSidebarProps) {
  const [domains, setDomains] = useState<string[]>([]);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ContextNode[]>([]);
  const [selectedNodes, setSelectedNodes] = useState<Set<string>>(new Set());
  const [isIndexing, setIsIndexing] = useState(false);
  const [indexStatus, setIndexStatus] = useState<string>("");
  const [isSearching, setIsSearching] = useState(false);

  // Load domains on mount
  useEffect(() => {
    loadDomains();

    // Listen for indexing progress
    const unsubProgress = window.codexApi.context.onIndexProgress((data) => {
      setIsIndexing(true);
      setIndexStatus(formatIndexStatus(data.status));
    });

    const unsubComplete = window.codexApi.context.onIndexComplete(
      (data: IndexCompleteNotification) => {
        setIsIndexing(false);
        setIndexStatus(
          `Complete: ${data.nodes_created} nodes from ${data.files_processed} files`
        );
        loadDomains();
        // Clear status after 5 seconds
        setTimeout(() => setIndexStatus(""), 5000);
      }
    );

    return () => {
      unsubProgress();
      unsubComplete();
    };
  }, []);

  const loadDomains = async () => {
    try {
      const result = await window.codexApi.context.listDomains();
      setDomains(result.domains);
    } catch (e) {
      console.error("Failed to load domains:", e);
    }
  };

  const handleSearch = useCallback(async () => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      return;
    }

    setIsSearching(true);
    try {
      const result = await window.codexApi.context.queryContext(
        searchQuery,
        50
      );
      setSearchResults(result.nodes);
    } catch (e) {
      console.error("Failed to search context:", e);
    } finally {
      setIsSearching(false);
    }
  }, [searchQuery]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") {
      handleSearch();
    }
  };

  const toggleNode = (nodeId: string) => {
    setSelectedNodes((prev) => {
      const next = new Set(prev);
      if (next.has(nodeId)) {
        next.delete(nodeId);
      } else {
        next.add(nodeId);
      }
      return next;
    });
  };

  const addToPrompt = () => {
    const nodes = searchResults.filter((n) => selectedNodes.has(n.id));
    onSelectNodes(nodes);
    setSelectedNodes(new Set());
  };

  const handleIndexDirectory = async () => {
    try {
      const result = await window.codexApi.fs.pickFolder();
      if (result && typeof result === "string") {
        setIndexStatus("Starting indexing...");
        setIsIndexing(true);
        await window.codexApi.context.indexDirectory(result);
      }
    } catch (e) {
      console.error("Failed to index directory:", e);
      setIsIndexing(false);
      setIndexStatus(`Error: ${e}`);
    }
  };

  return (
    <div style={styles.sidebar}>
      <div style={styles.header}>
        <h3 style={styles.title}>Context Tree</h3>
        <button onClick={onClose} style={styles.closeButton}>
          &times;
        </button>
      </div>

      {/* Indexing Status */}
      {(isIndexing || indexStatus) && (
        <div
          style={{
            ...styles.statusBox,
            backgroundColor: isIndexing ? "#1a365d" : "#1c4532",
          }}
        >
          {isIndexing && <span style={styles.spinner}>&#8987;</span>}
          <span>{indexStatus}</span>
        </div>
      )}

      {/* Index Button */}
      <button
        onClick={handleIndexDirectory}
        disabled={isIndexing}
        style={{
          ...styles.indexButton,
          opacity: isIndexing ? 0.5 : 1,
        }}
      >
        {isIndexing ? "Indexing..." : "Index Directory (Cmd+K)"}
      </button>

      {/* Domains Section */}
      <div style={styles.section}>
        <h4 style={styles.sectionTitle}>Indexed Domains</h4>
        {domains.length === 0 ? (
          <p style={styles.emptyText}>
            No domains indexed yet. Press Cmd+K to index a directory.
          </p>
        ) : (
          <ul style={styles.domainList}>
            {domains.map((domain) => (
              <li key={domain} style={styles.domainItem}>
                {domain}
              </li>
            ))}
          </ul>
        )}
      </div>

      {/* Search Section */}
      <div style={styles.section}>
        <h4 style={styles.sectionTitle}>Search Context</h4>
        <div style={styles.searchBox}>
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search files, topics, entities..."
            style={styles.searchInput}
          />
          <button
            onClick={handleSearch}
            disabled={isSearching}
            style={styles.searchButton}
          >
            {isSearching ? "..." : "Search"}
          </button>
        </div>
      </div>

      {/* Results Section */}
      {searchResults.length > 0 && (
        <div style={styles.section}>
          <h4 style={styles.sectionTitle}>Results ({searchResults.length})</h4>
          <div style={styles.resultsList}>
            {searchResults.map((node) => (
              <div
                key={node.id}
                style={{
                  ...styles.resultItem,
                  backgroundColor: selectedNodes.has(node.id)
                    ? "#2d3748"
                    : "transparent",
                }}
                onClick={() => toggleNode(node.id)}
              >
                <input
                  type="checkbox"
                  checked={selectedNodes.has(node.id)}
                  onChange={() => toggleNode(node.id)}
                  style={styles.checkbox}
                />
                <div style={styles.resultContent}>
                  <div style={styles.resultHeader}>
                    <span style={styles.nodeType}>{node.node_type}</span>
                    <span style={styles.nodeName}>{node.name}</span>
                  </div>
                  <div style={styles.resultSummary}>
                    {node.summary.slice(0, 150)}
                    {node.summary.length > 150 ? "..." : ""}
                  </div>
                  {node.path && (
                    <div style={styles.resultPath}>{node.path}</div>
                  )}
                  {node.keywords.length > 0 && (
                    <div style={styles.resultKeywords}>
                      {node.keywords.slice(0, 5).join(", ")}
                    </div>
                  )}
                </div>
              </div>
            ))}
          </div>
          <button
            onClick={addToPrompt}
            disabled={selectedNodes.size === 0}
            style={{
              ...styles.addButton,
              opacity: selectedNodes.size === 0 ? 0.5 : 1,
            }}
          >
            Add {selectedNodes.size} to prompt
          </button>
        </div>
      )}
    </div>
  );
}

function formatIndexStatus(status: IndexStatus): string {
  switch (status.type) {
    case "starting":
      return `Starting: ${status.path}`;
    case "processing":
      return `Processing ${status.file} (${status.progress}/${status.total})`;
    case "analyzing":
      return `Analyzing: ${status.stage}`;
    case "complete":
      return `Complete: ${status.nodes_created} nodes`;
    case "error":
      return `Error: ${status.message}`;
    default:
      return "Processing...";
  }
}

const styles: Record<string, React.CSSProperties> = {
  sidebar: {
    width: "320px",
    minWidth: "320px",
    height: "100vh",
    backgroundColor: "#1a1a2e",
    borderRight: "1px solid #333",
    display: "flex",
    flexDirection: "column",
    overflow: "auto",
    zIndex: 100,
    position: "relative",
  },
  header: {
    display: "flex",
    justifyContent: "space-between",
    alignItems: "center",
    padding: "12px 16px",
    borderBottom: "1px solid #333",
  },
  title: {
    margin: 0,
    fontSize: "16px",
    fontWeight: 600,
    color: "#e2e8f0",
  },
  closeButton: {
    background: "none",
    border: "none",
    color: "#a0aec0",
    fontSize: "20px",
    cursor: "pointer",
    padding: "4px 8px",
  },
  statusBox: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    padding: "8px 16px",
    fontSize: "12px",
    color: "#e2e8f0",
  },
  spinner: {
    animation: "spin 1s linear infinite",
  },
  indexButton: {
    margin: "12px 16px",
    padding: "10px 16px",
    backgroundColor: "#4a5568",
    color: "#e2e8f0",
    border: "none",
    borderRadius: "6px",
    cursor: "pointer",
    fontSize: "14px",
    fontWeight: 500,
  },
  section: {
    padding: "12px 16px",
    borderTop: "1px solid #333",
  },
  sectionTitle: {
    margin: "0 0 8px 0",
    fontSize: "13px",
    fontWeight: 600,
    color: "#a0aec0",
    textTransform: "uppercase",
    letterSpacing: "0.5px",
  },
  emptyText: {
    fontSize: "13px",
    color: "#718096",
    margin: 0,
  },
  domainList: {
    listStyle: "none",
    margin: 0,
    padding: 0,
  },
  domainItem: {
    padding: "6px 12px",
    fontSize: "13px",
    color: "#e2e8f0",
    backgroundColor: "#2d3748",
    borderRadius: "4px",
    marginBottom: "4px",
  },
  searchBox: {
    display: "flex",
    gap: "8px",
  },
  searchInput: {
    flex: 1,
    padding: "8px 12px",
    backgroundColor: "#2d3748",
    border: "1px solid #4a5568",
    borderRadius: "4px",
    color: "#e2e8f0",
    fontSize: "13px",
  },
  searchButton: {
    padding: "8px 16px",
    backgroundColor: "#3182ce",
    color: "white",
    border: "none",
    borderRadius: "4px",
    cursor: "pointer",
    fontSize: "13px",
  },
  resultsList: {
    maxHeight: "300px",
    overflowY: "auto",
    marginBottom: "12px",
  },
  resultItem: {
    display: "flex",
    alignItems: "flex-start",
    gap: "8px",
    padding: "10px",
    borderRadius: "4px",
    cursor: "pointer",
    marginBottom: "4px",
  },
  checkbox: {
    marginTop: "4px",
  },
  resultContent: {
    flex: 1,
    minWidth: 0,
  },
  resultHeader: {
    display: "flex",
    alignItems: "center",
    gap: "8px",
    marginBottom: "4px",
  },
  nodeType: {
    fontSize: "10px",
    padding: "2px 6px",
    backgroundColor: "#4a5568",
    borderRadius: "3px",
    color: "#a0aec0",
    textTransform: "uppercase",
  },
  nodeName: {
    fontSize: "13px",
    fontWeight: 500,
    color: "#e2e8f0",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  resultSummary: {
    fontSize: "12px",
    color: "#a0aec0",
    lineHeight: 1.4,
    marginBottom: "4px",
  },
  resultPath: {
    fontSize: "11px",
    color: "#718096",
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap",
  },
  resultKeywords: {
    fontSize: "11px",
    color: "#63b3ed",
    marginTop: "4px",
  },
  addButton: {
    width: "100%",
    padding: "10px 16px",
    backgroundColor: "#38a169",
    color: "white",
    border: "none",
    borderRadius: "6px",
    cursor: "pointer",
    fontSize: "14px",
    fontWeight: 500,
  },
};
