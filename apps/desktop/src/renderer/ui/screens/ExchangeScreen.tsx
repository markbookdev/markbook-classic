import React, { useState } from "react";
import {
  ExchangeApplyClassCsvResultSchema,
  ExchangeExportClassCsvResultSchema,
  ExchangePreviewClassCsvResultSchema,
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
  const [preview, setPreview] = useState<{
    rowsTotal: number;
    rowsParsed: number;
    rowsMatched: number;
    rowsUnmatched: number;
    warningsCount: number;
  } | null>(null);

  async function browseExportPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickSave({
        title: "Export Class CSV",
        defaultPath: "class-exchange.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setExportPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseImportPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickOpen({
        title: "Import Class CSV",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setImportPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

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

  async function previewCsv() {
    if (!importPath.trim()) {
      props.onError("Enter an import path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.previewClassCsv",
        {
          classId: props.selectedClassId,
          inPath: importPath.trim(),
          mode
        },
        ExchangePreviewClassCsvResultSchema
      );
      setPreview({
        rowsTotal: res.rowsTotal,
        rowsParsed: res.rowsParsed,
        rowsMatched: res.rowsMatched,
        rowsUnmatched: res.rowsUnmatched,
        warningsCount: res.warningsCount
      });
      setStatus(`Preview: matched ${res.rowsMatched}/${res.rowsParsed} parsed rows.`);
    } catch (e: any) {
      setPreview(null);
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
        "exchange.applyClassCsv",
        {
          classId: props.selectedClassId,
          inPath: importPath.trim(),
          mode
        },
        ExchangeApplyClassCsvResultSchema
      );
      setStatus(
        `Applied ${res.updated} score rows (${mode}); skipped ${res.skipped}, warnings ${res.warningsCount}.`
      );
      setPreview({
        rowsTotal: res.rowsTotal,
        rowsParsed: res.rowsParsed,
        rowsMatched: res.updated,
        rowsUnmatched: res.skipped,
        warningsCount: res.warningsCount
      });
    } catch (e: any) {
      // Fall back to legacy import endpoint for older sidecars.
      try {
        const fallback = await requestParsed(
          "exchange.importClassCsv",
          {
            classId: props.selectedClassId,
            inPath: importPath.trim(),
            mode
          },
          ExchangeImportClassCsvResultSchema
        );
        setStatus(`Imported ${fallback.updated} score rows (${mode}).`);
      } catch (fallbackErr: any) {
        props.onError(fallbackErr?.message ?? String(fallbackErr));
      }
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
          <button
            data-testid="exchange-export-browse-btn"
            disabled={busy}
            onClick={() => void browseExportPath()}
          >
            Browse
          </button>
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
          <button
            data-testid="exchange-import-browse-btn"
            disabled={busy}
            onClick={() => void browseImportPath()}
          >
            Browse
          </button>
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
          <button data-testid="exchange-preview-btn" disabled={busy} onClick={() => void previewCsv()}>
            {busy ? "Working..." : "Preview"}
          </button>
          <button data-testid="exchange-import-btn" disabled={busy} onClick={() => void importCsv()}>
            {busy ? "Working..." : "Import"}
          </button>
        </div>
      </div>

      {preview ? (
        <div
          data-testid="exchange-preview-summary"
          style={{ marginTop: 8, color: "#444", fontSize: 13 }}
        >
          Parsed {preview.rowsParsed}/{preview.rowsTotal}, matched {preview.rowsMatched}, unmatched{" "}
          {preview.rowsUnmatched}, warnings {preview.warningsCount}
        </div>
      ) : null}
      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
      <div style={{ marginTop: 10, color: "#666", fontSize: 12 }}>
        CSV format is the MarkBook desktop exchange format emitted by this app.
      </div>
    </div>
  );
}
