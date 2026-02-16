export {};

declare global {
  interface Window {
    markbook: {
      selectWorkspace: () => Promise<string | null>;
      selectLegacyClassFolder: () => Promise<string | null>;
      request: (method: string, params?: Record<string, unknown>) => Promise<any>;
      restartSidecar: () => Promise<{ ok: true }>;
      getSidecarMeta: () => Promise<{
        running: boolean;
        pid: number | null;
        path: string | null;
      }>;
      prefs: {
        get: () => Promise<{
          recentWorkspaces: string[];
          lastWorkspace: string | null;
        }>;
        addRecentWorkspace: (path: string) => Promise<{
          ok: true;
          prefs: { recentWorkspaces: string[]; lastWorkspace: string | null };
        }>;
        setLastWorkspace: (path: string) => Promise<{
          ok: true;
          prefs: { recentWorkspaces: string[]; lastWorkspace: string | null };
        }>;
      };
      exportPdfHtml: (html: string, outPath: string) => Promise<{ ok: true }>;
      exportPdfHtmlWithSaveDialog: (
        html: string,
        defaultFilename: string
      ) => Promise<{ canceled: boolean; path?: string }>;
    };
  }
}
