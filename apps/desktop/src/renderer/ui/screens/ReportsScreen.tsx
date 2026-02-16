import React, { useState } from "react";
import {
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml
} from "@markbook/reports";
import {
  ReportsMarkSetGridModelResultSchema,
  ReportsMarkSetSummaryModelResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

function sanitizeFilename(name: string) {
  // Keep it simple and cross-platform.
  return name.replace(/[\\/:*?\"<>|]/g, "-").replace(/\s+/g, " ").trim();
}

export function ReportsScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
}) {
  const [exportingGridPdf, setExportingGridPdf] = useState(false);
  const [exportingSummaryPdf, setExportingSummaryPdf] = useState(false);

  async function exportMarkSetGridPdf() {
    setExportingGridPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.markSetGridModel",
        { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
        ReportsMarkSetGridModelResultSchema
      );
      const html = renderMarkSetGridReportHtml(model);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - ${model.markSet.code} - Grid.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingGridPdf(false);
    }
  }

  async function exportMarkSetSummaryPdf() {
    setExportingSummaryPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.markSetSummaryModel",
        { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
        ReportsMarkSetSummaryModelResultSchema
      );
      const html = renderMarkSetSummaryReportHtml(model);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - ${model.markSet.code} - Summary.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingSummaryPdf(false);
    }
  }

  return (
    <div data-testid="reports-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 8 }}>Reports</div>
      <div style={{ color: "#444", marginBottom: 8 }}>Print Mark Set Grid</div>
      <button
        data-testid="export-markset-grid-pdf-btn"
        onClick={() => void exportMarkSetGridPdf()}
        disabled={exportingGridPdf || exportingSummaryPdf}
      >
        {exportingGridPdf ? "Exporting..." : "Export Grid PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Mark Set Summary</div>
      <button
        data-testid="export-markset-summary-pdf-btn"
        onClick={() => void exportMarkSetSummaryPdf()}
        disabled={exportingGridPdf || exportingSummaryPdf}
      >
        {exportingSummaryPdf ? "Exporting..." : "Export Summary PDF"}
      </button>
      <div style={{ marginTop: 12, fontSize: 12, color: "#666" }}>
        Uses Chromium print-to-PDF and preserves legacy mark semantics (blank = No Mark, 0 = Zero).
      </div>
    </div>
  );
}
