export type FileEntry = {
  name: string;
  path: string;
  isDir: boolean;
  size: number;
  mtimeMs: number;
};

type FavoriteEntry = {
  label: string;
  path: string;
};

type CodexEvent = {
  id?: string;
  msg: {
    type: string;
    [key: string]: unknown;
  };
};

type ApprovalEvent = {
  id: number;
  method: "applyPatchApproval" | "execCommandApproval";
  params: Record<string, unknown>;
};

// Context system types
export type ContextNode = {
  id: string;
  name: string;
  node_type: string;
  path?: string;
  summary: string;
  depth: number;
  keywords: string[];
};

export type QueryContextResult = {
  nodes: ContextNode[];
  processing_time_ms: number;
};

export type NodeContextResult = {
  node: ContextNode;
  ancestry: ContextNode[];
  related: ContextNode[];
};

export type IndexStatus =
  | { type: "starting"; path: string }
  | { type: "processing"; file: string; progress: number; total: number }
  | { type: "analyzing"; stage: string }
  | { type: "complete"; nodes_created: number; files_processed: number }
  | { type: "error"; message: string };

export type IndexDirectoryResult = {
  started: boolean;
  path: string;
};

export type IndexCompleteNotification = {
  domain: string;
  files_processed: number;
  nodes_created: number;
  entities_extracted: number;
  processing_time_ms: number;
};

export type ListDomainsResult = {
  domains: string[];
};

declare global {
  interface Window {
    codexApi: {
      fs: {
        listDirectory: (dirPath: string) => Promise<FileEntry[]>;
        searchDirectory: (payload: {
          root: string;
          query: string;
          recursive: boolean;
        }) => Promise<FileEntry[]>;
        getFavorites: () => Promise<FavoriteEntry[]>;
        pickFolder: () => Promise<string | null>;
        openPath: (targetPath: string) => Promise<void>;
        revealInFinder: (targetPath: string) => Promise<void>;
        openWith: (targetPath: string) => Promise<{ canceled: boolean; appPath?: string }>;
        checkAgentsMd: (dirPath: string) => Promise<boolean>;
      };
      codex: {
        start: (options?: {
          model?: string;
          cwd?: string;
          approvalPolicy?: string;
          sandbox?: string;
          command?: string;
          args?: string[];
        }) => Promise<{ conversationId: string }>;
        send: (payload: { text: string; cwd: string }) => Promise<void>;
        approve: (payload: { id: number; decision: "allow" | "deny" }) => Promise<void>;
        stop: () => Promise<void>;
        account: {
          read: (params?: { refreshToken?: boolean }) => Promise<{
            account: { type: "apiKey" } | { type: "chatgpt"; email: string; planType: string } | null;
            requiresOpenaiAuth: boolean;
          }>;
          loginStart: (params: { type: "apiKey"; apiKey: string } | { type: "chatgpt" }) => Promise<{
            type: "apiKey" | "chatgpt";
            loginId?: string;
            authUrl?: string;
          }>;
          loginCancel: (params: { loginId: string }) => Promise<void>;
          logout: () => Promise<void>;
        };
        onEvent: (handler: (event: CodexEvent) => void) => () => void;
        onApproval: (handler: (event: ApprovalEvent) => void) => () => void;
        onStatus: (handler: (event: { type: string; [key: string]: unknown }) => void) => () => void;
        onReady: (handler: (event: { conversationId: string }) => void) => () => void;
      };
      context: {
        indexDirectory: (path: string) => Promise<IndexDirectoryResult>;
        queryContext: (query: string, maxResults?: number) => Promise<QueryContextResult>;
        getNodeContext: (nodeId: string) => Promise<NodeContextResult>;
        listDomains: () => Promise<ListDomainsResult>;
        onIndexProgress: (handler: (data: { status: IndexStatus }) => void) => () => void;
        onIndexComplete: (handler: (data: IndexCompleteNotification) => void) => () => void;
      };
    };
  }
}
