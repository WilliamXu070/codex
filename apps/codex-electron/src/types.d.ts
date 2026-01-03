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
      };
    };
  }
}
