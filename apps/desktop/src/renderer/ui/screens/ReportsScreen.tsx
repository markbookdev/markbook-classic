import React, { useState } from "react";
import {
  renderCategoryAnalysisReportHtml,
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderStudentSummaryReportHtml
} from "@markbook/reports";
import {
  MarkSetOpenResultSchema,
  ReportsCategoryAnalysisModelResultSchema,
  ReportsMarkSetGridModelResultSchema,
  ReportsMarkSetSummaryModelResultSchema,
  ReportsStudentSummaryModelResultSchema
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
  const [exportingCategoryPdf, setExportingCategoryPdf] = useState(false);
  const [exportingStudentPdf, setExportingStudentPdf] = useState(false);
  const [students, setStudents] = useState<Array<{ id: string; displayName: string }>>([]);
  const [selectedStudentId, setSelectedStudentId] = useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    async function loadStudents() {
      try {
        const open = await requestParsed(
          "markset.open",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          MarkSetOpenResultSchema
        );
        if (cancelled) return;
        setStudents(open.students.map((s) => ({ id: s.id, displayName: s.displayName })));
        setSelectedStudentId((cur) => {
          if (cur && open.students.some((s) => s.id === cur)) return cur;
          return open.students[0]?.id ?? null;
        });
      } catch {
        if (cancelled) return;
        setStudents([]);
        setSelectedStudentId(null);
      }
    }
    void loadStudents();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, props.selectedMarkSetId]);

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

  async function exportCategoryAnalysisPdf() {
    setExportingCategoryPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.categoryAnalysisModel",
        { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
        ReportsCategoryAnalysisModelResultSchema
      );
      const html = renderCategoryAnalysisReportHtml(model);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - ${model.markSet.code} - Category Analysis.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingCategoryPdf(false);
    }
  }

  async function exportStudentSummaryPdf() {
    if (!selectedStudentId) return;
    setExportingStudentPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.studentSummaryModel",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          studentId: selectedStudentId
        },
        ReportsStudentSummaryModelResultSchema
      );
      const html = renderStudentSummaryReportHtml(model);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - ${model.markSet.code} - ${model.student.displayName} - Summary.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingStudentPdf(false);
    }
  }

  return (
    <div data-testid="reports-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 8 }}>Reports</div>
      <div style={{ color: "#444", marginBottom: 8 }}>Print Mark Set Grid</div>
      <button
        data-testid="export-markset-grid-pdf-btn"
        onClick={() => void exportMarkSetGridPdf()}
        disabled={exportingGridPdf || exportingSummaryPdf || exportingCategoryPdf || exportingStudentPdf}
      >
        {exportingGridPdf ? "Exporting..." : "Export Grid PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Mark Set Summary</div>
      <button
        data-testid="export-markset-summary-pdf-btn"
        onClick={() => void exportMarkSetSummaryPdf()}
        disabled={exportingGridPdf || exportingSummaryPdf || exportingCategoryPdf || exportingStudentPdf}
      >
        {exportingSummaryPdf ? "Exporting..." : "Export Summary PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Category Analysis</div>
      <button
        data-testid="export-category-analysis-pdf-btn"
        onClick={() => void exportCategoryAnalysisPdf()}
        disabled={exportingGridPdf || exportingSummaryPdf || exportingCategoryPdf || exportingStudentPdf}
      >
        {exportingCategoryPdf ? "Exporting..." : "Export Category Analysis PDF"}
      </button>

      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>
        Student Summary (selected student)
      </div>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <select
          data-testid="student-summary-select"
          value={selectedStudentId ?? ""}
          onChange={(e) => setSelectedStudentId(e.currentTarget.value || null)}
        >
          {students.map((s) => (
            <option key={s.id} value={s.id}>
              {s.displayName}
            </option>
          ))}
        </select>
        <button
          data-testid="export-student-summary-pdf-btn"
          onClick={() => void exportStudentSummaryPdf()}
          disabled={
            !selectedStudentId ||
            exportingGridPdf ||
            exportingSummaryPdf ||
            exportingCategoryPdf ||
            exportingStudentPdf
          }
        >
          {exportingStudentPdf ? "Exporting..." : "Export Student Summary PDF"}
        </button>
      </div>
      <div style={{ marginTop: 12, fontSize: 12, color: "#666" }}>
        Uses Chromium print-to-PDF and preserves legacy mark semantics (blank = No Mark, 0 = Zero).
      </div>
    </div>
  );
}
