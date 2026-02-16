// E2E hooks for Playwright Electron tests.
//
// These are intentionally not part of the supported app API. Keep them under
// window.__markbookTest so production users never rely on them.
import {
  renderCategoryAnalysisReportHtml,
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderStudentSummaryReportHtml
} from "@markbook/reports";

declare global {
  interface Window {
    __markbookTest?: any;
  }
}

function ensure() {
  if (!window.__markbookTest) window.__markbookTest = {};
  return window.__markbookTest;
}

const t = ensure();

// Render report HTML in the renderer bundle (so we don't need Node TS/ESM support in tests).
t.renderMarkSetGridReportHtml = renderMarkSetGridReportHtml;

// Export a mark set grid report to a specific PDF path without a Save dialog.
t.exportMarkSetGridPdfToPath = async (classId: string, markSetId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.markSetGridModel", { classId, markSetId });
  const html = renderMarkSetGridReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportMarkSetSummaryPdfToPath = async (
  classId: string,
  markSetId: string,
  outPath: string
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.markSetSummaryModel", {
    classId,
    markSetId
  });
  const html = renderMarkSetSummaryReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportCategoryAnalysisPdfToPath = async (
  classId: string,
  markSetId: string,
  outPath: string
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.categoryAnalysisModel", {
    classId,
    markSetId
  });
  const html = renderCategoryAnalysisReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportStudentSummaryPdfToPath = async (
  classId: string,
  markSetId: string,
  studentId: string,
  outPath: string
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.studentSummaryModel", {
    classId,
    markSetId,
    studentId
  });
  const html = renderStudentSummaryReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.attendanceSetStudentDay = async (
  classId: string,
  month: string,
  studentId: string,
  day: number,
  code: string | null
) => {
  return await window.markbook.request("attendance.setStudentDay", {
    classId,
    month,
    studentId,
    day,
    code
  });
};

t.seatingSavePlan = async (
  classId: string,
  rows: number,
  seatsPerRow: number,
  blockedSeatCodes: number[],
  assignments: Array<number | null>
) => {
  return await window.markbook.request("seating.save", {
    classId,
    rows,
    seatsPerRow,
    blockedSeatCodes,
    assignments
  });
};
