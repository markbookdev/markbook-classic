// E2E hooks for Playwright Electron tests.
//
// These are intentionally not part of the supported app API. Keep them under
// window.__markbookTest so production users never rely on them.
import {
  renderAttendanceMonthlyReportHtml,
  renderCourseDescriptionReportHtml,
  renderClassListReportHtml,
  renderCategoryAnalysisReportHtml,
  renderCombinedAnalysisReportHtml,
  renderLearningSkillsSummaryReportHtml,
  renderPlannerLessonReportHtml,
  renderPlannerUnitReportHtml,
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderStudentSummaryReportHtml,
  renderTimeManagementReportHtml
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
t.exportMarkSetGridPdfToPath = async (
  classId: string,
  markSetId: string,
  outPath: string,
  options?: {
    filters?: { term?: number | null; categoryName?: string | null; typesMask?: number | null };
    studentScope?: "all" | "active" | "valid";
  }
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.markSetGridModel", {
    classId,
    markSetId,
    ...(options?.filters ? { filters: options.filters } : {}),
    ...(options?.studentScope ? { studentScope: options.studentScope } : {})
  });
  const html = renderMarkSetGridReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportMarkSetSummaryPdfToPath = async (
  classId: string,
  markSetId: string,
  outPath: string,
  options?: {
    filters?: { term?: number | null; categoryName?: string | null; typesMask?: number | null };
    studentScope?: "all" | "active" | "valid";
  }
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.markSetSummaryModel", {
    classId,
    markSetId,
    ...(options?.filters ? { filters: options.filters } : {}),
    ...(options?.studentScope ? { studentScope: options.studentScope } : {})
  });
  const html = renderMarkSetSummaryReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportCategoryAnalysisPdfToPath = async (
  classId: string,
  markSetId: string,
  outPath: string,
  options?: {
    filters?: { term?: number | null; categoryName?: string | null; typesMask?: number | null };
    studentScope?: "all" | "active" | "valid";
  }
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.categoryAnalysisModel", {
    classId,
    markSetId,
    ...(options?.filters ? { filters: options.filters } : {}),
    ...(options?.studentScope ? { studentScope: options.studentScope } : {})
  });
  const html = renderCategoryAnalysisReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportStudentSummaryPdfToPath = async (
  classId: string,
  markSetId: string,
  studentId: string,
  outPath: string,
  options?: {
    filters?: { term?: number | null; categoryName?: string | null; typesMask?: number | null };
    studentScope?: "all" | "active" | "valid";
  }
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.studentSummaryModel", {
    classId,
    markSetId,
    studentId,
    ...(options?.filters ? { filters: options.filters } : {}),
    ...(options?.studentScope ? { studentScope: options.studentScope } : {})
  });
  const html = renderStudentSummaryReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportCombinedAnalysisPdfToPath = async (
  classId: string,
  markSetIds: string[],
  outPath: string,
  options?: {
    filters?: { term?: number | null; categoryName?: string | null; typesMask?: number | null };
    studentScope?: "all" | "active" | "valid";
  }
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.combinedAnalysisModel", {
    classId,
    markSetIds,
    ...(options?.filters ? { filters: options.filters } : {}),
    ...(options?.studentScope ? { studentScope: options.studentScope } : {})
  });
  const html = renderCombinedAnalysisReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportAttendanceMonthlyPdfToPath = async (
  classId: string,
  month: string,
  outPath: string
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.attendanceMonthlyModel", {
    classId,
    month
  });
  const html = renderAttendanceMonthlyReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportClassListPdfToPath = async (classId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.classListModel", { classId });
  const html = renderClassListReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportLearningSkillsSummaryPdfToPath = async (
  classId: string,
  term: number,
  outPath: string
) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.learningSkillsSummaryModel", {
    classId,
    term
  });
  const html = renderLearningSkillsSummaryReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportPlannerUnitPdfToPath = async (classId: string, unitId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.plannerUnitModel", {
    classId,
    unitId
  });
  const html = renderPlannerUnitReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportPlannerLessonPdfToPath = async (classId: string, lessonId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.plannerLessonModel", {
    classId,
    lessonId
  });
  const html = renderPlannerLessonReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportCourseDescriptionPdfToPath = async (classId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.courseDescriptionModel", {
    classId
  });
  const html = renderCourseDescriptionReportHtml(model);
  await window.markbook.exportPdfHtml(html, outPath);
  return { ok: true };
};

t.exportTimeManagementPdfToPath = async (classId: string, outPath: string) => {
  if (!window.markbook?.request) throw new Error("window.markbook.request missing");
  if (!window.markbook?.exportPdfHtml) throw new Error("window.markbook.exportPdfHtml missing");
  const model = await window.markbook.request("reports.timeManagementModel", {
    classId
  });
  const html = renderTimeManagementReportHtml(model);
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
