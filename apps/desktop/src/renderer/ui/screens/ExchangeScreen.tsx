import React, { useState } from "react";
import {
  ExchangeExportClassCsvResultSchema,
  ExchangeImportClassCsvResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

export function ExchangeScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [mode, setMode] = useState<"upsert" | "replace">("upsert");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");

  async function exportCsv() {
    if (!exportPath.trim()) {
      props.onError("Enter an export path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.exportClassCsv",
        { classId: props.selectedClassId, outPath: exportPath.trim() },
        ExchangeExportClassCsvResultSchema
      );
      setStatus(`Exported ${res.rowsExported} rows to ${res.path}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function importCsv() {
    if (!importPath.trim()) {
      props.onError("Enter an import path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.importClassCsv",
        {
          classId: props.selectedClassId,
          inPath: importPath.trim(),
          mode
        },
        ExchangeImportClassCsvResultSchema
      );
      setStatus(`Imported ${res.updated} score rows (${mode}).`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div data-testid="exchange-screen" style={{ padding: 24, maxWidth: 920 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Exchange</div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14, marginBottom: 12 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Export Class CSV</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <input
            data-testid="exchange-export-path-input"
            value={exportPath}
            onChange={(e) => setExportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/class-export.csv"
            style={{ flex: "1 1 560px", padding: "6px 8px" }}
          />
          <button data-testid="exchange-export-btn" disabled={busy} onClick={() => void exportCsv()}>
            {busy ? "Working..." : "Export"}
          </button>
        </div>
      </div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Import Class CSV</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
          <input
            data-testid="exchange-import-path-input"
            value={importPath}
            onChange={(e) => setImportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/class-export.csv"
            style={{ flex: "1 1 560px", padding: "6px 8px" }}
          />
          <label>
            Mode{" "}
            <select
              data-testid="exchange-import-mode-select"
              value={mode}
              onChange={(e) => setMode((e.currentTarget.value as any) || "upsert")}
            >
              <option value="upsert">Upsert</option>
              <option value="replace">Replace</option>
            </select>
          </label>
          <button data-testid="exchange-import-btn" disabled={busy} onClick={() => void importCsv()}>
            {busy ? "Working..." : "Import"}
          </button>
        </div>
      </div>

      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
      <div style={{ marginTop: 10, color: "#666", fontSize: 12 }}>
        CSV format is the MarkBook desktop exchange format emitted by this app.
      </div>
    </div>
  );
}
