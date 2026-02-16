import React, { useState } from "react";
import {
  BackupExportWorkspaceBundleResultSchema,
  BackupImportWorkspaceBundleResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

export function BackupScreen(props: {
  workspacePath: string | null;
  onError: (msg: string | null) => void;
  onAfterImport?: () => void | Promise<void>;
}) {
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");

  async function exportBundle() {
    if (!exportPath.trim()) {
      props.onError("Enter an output file path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "backup.exportWorkspaceBundle",
        {
          workspacePath: props.workspacePath,
          outPath: exportPath.trim()
        },
        BackupExportWorkspaceBundleResultSchema
      );
      setStatus(`Exported backup to ${res.path}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function importBundle() {
    if (!importPath.trim()) {
      props.onError("Enter an input file path first.");
      return;
    }
    const ok = confirm("Import backup into current workspace? This replaces markbook.sqlite3.");
    if (!ok) return;

    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "backup.importWorkspaceBundle",
        {
          inPath: importPath.trim(),
          workspacePath: props.workspacePath
        },
        BackupImportWorkspaceBundleResultSchema
      );
      setStatus(`Imported backup${res.workspacePath ? ` into ${res.workspacePath}` : ""}`);
      await props.onAfterImport?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div data-testid="backup-screen" style={{ padding: 24, maxWidth: 880 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Backup / Restore</div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14, marginBottom: 12 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Export Workspace Bundle</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <input
            data-testid="backup-export-path-input"
            value={exportPath}
            onChange={(e) => setExportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/backup.markbook.sqlite3"
            style={{ flex: "1 1 520px", padding: "6px 8px" }}
          />
          <button data-testid="backup-export-btn" disabled={busy} onClick={() => void exportBundle()}>
            {busy ? "Working..." : "Export"}
          </button>
        </div>
      </div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Import Workspace Bundle</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <input
            data-testid="backup-import-path-input"
            value={importPath}
            onChange={(e) => setImportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/backup.markbook.sqlite3"
            style={{ flex: "1 1 520px", padding: "6px 8px" }}
          />
          <button data-testid="backup-import-btn" disabled={busy} onClick={() => void importBundle()}>
            {busy ? "Working..." : "Import"}
          </button>
        </div>
      </div>

      <div style={{ marginTop: 12, color: "#666", fontSize: 12 }}>
        Current workspace: {props.workspacePath ?? "(none)"}
      </div>
      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
    </div>
  );
}
