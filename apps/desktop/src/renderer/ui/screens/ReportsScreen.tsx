import React, { useState } from "react";
import {
  renderAttendanceMonthlyReportHtml,
  renderClassListReportHtml,
  renderCategoryAnalysisReportHtml,
  renderCombinedAnalysisReportHtml,
  renderLearningSkillsSummaryReportHtml,
  renderMarkSetGridReportHtml,
  renderMarkSetSummaryReportHtml,
  renderStudentSummaryReportHtml
} from "@markbook/reports";
import {
  MarkSetOpenResultSchema,
  ReportsAttendanceMonthlyModelResultSchema,
  ReportsClassListModelResultSchema,
  ReportsCategoryAnalysisModelResultSchema,
  ReportsCombinedAnalysisModelResultSchema,
  ReportsLearningSkillsSummaryModelResultSchema,
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
  initialContext?: {
    filters: { term: number | null; categoryName: string | null; typesMask: number | null };
    studentScope: "all" | "active" | "valid";
    studentId?: string | null;
    markSetIds?: string[] | null;
  };
  contextVersion?: number;
}) {
  const [exportingGridPdf, setExportingGridPdf] = useState(false);
  const [exportingSummaryPdf, setExportingSummaryPdf] = useState(false);
  const [exportingCategoryPdf, setExportingCategoryPdf] = useState(false);
  const [exportingCombinedPdf, setExportingCombinedPdf] = useState(false);
  const [exportingStudentPdf, setExportingStudentPdf] = useState(false);
  const [exportingAttendancePdf, setExportingAttendancePdf] = useState(false);
  const [exportingClassListPdf, setExportingClassListPdf] = useState(false);
  const [exportingLearningSkillsPdf, setExportingLearningSkillsPdf] = useState(false);
  const [students, setStudents] = useState<Array<{ id: string; displayName: string }>>([]);
  const [categoryOptions, setCategoryOptions] = useState<string[]>([]);
  const [selectedStudentId, setSelectedStudentId] = useState<string | null>(null);
  const [combinedMarkSetIds, setCombinedMarkSetIds] = useState<string[] | null>(null);
  const [studentScope, setStudentScope] = useState<"all" | "active" | "valid">("all");
  const [reportFilters, setReportFilters] = useState<{
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  }>({
    term: null,
    categoryName: null,
    typesMask: null
  });
  const [typesSelected, setTypesSelected] = useState<[boolean, boolean, boolean, boolean, boolean]>(
    [true, true, true, true, true]
  );
  const [attendanceMonth, setAttendanceMonth] = useState<string>(
    new Date().toISOString().slice(0, 7)
  );
  const [learningSkillsTerm, setLearningSkillsTerm] = useState<number>(1);

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
        const cats = Array.from(
          new Set(
            open.assessments
              .map((a) => a.categoryName ?? "")
              .map((s) => s.trim())
              .filter((s) => s.length > 0)
          )
        ).sort((a, b) => a.localeCompare(b));
        setCategoryOptions(cats);
        setSelectedStudentId((cur) => {
          if (cur && open.students.some((s) => s.id === cur)) return cur;
          return open.students[0]?.id ?? null;
        });
      } catch {
        if (cancelled) return;
        setStudents([]);
        setCategoryOptions([]);
        setSelectedStudentId(null);
      }
    }
    void loadStudents();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, props.selectedMarkSetId]);

  React.useEffect(() => {
    if (!props.initialContext) return;
    setReportFilters({
      term: props.initialContext.filters.term ?? null,
      categoryName: props.initialContext.filters.categoryName ?? null,
      typesMask: props.initialContext.filters.typesMask ?? null
    });
    const mask = props.initialContext.filters.typesMask;
    if (mask == null) {
      setTypesSelected([true, true, true, true, true]);
    } else {
      setTypesSelected([
        (mask & 1) !== 0,
        (mask & 2) !== 0,
        (mask & 4) !== 0,
        (mask & 8) !== 0,
        (mask & 16) !== 0
      ]);
    }
    setStudentScope(props.initialContext.studentScope ?? "all");
    if (props.initialContext.studentId) {
      setSelectedStudentId(props.initialContext.studentId);
    }
    const fromCombined = props.initialContext.markSetIds ?? null;
    setCombinedMarkSetIds(
      Array.isArray(fromCombined) && fromCombined.length > 0 ? [...fromCombined] : null
    );
  }, [props.contextVersion, props.initialContext]);

  React.useEffect(() => {
    let mask = 0;
    for (let i = 0; i < typesSelected.length; i += 1) {
      if (typesSelected[i]) mask |= 1 << i;
    }
    setReportFilters((cur) => ({
      ...cur,
      typesMask: mask === 0 || mask === 31 ? null : mask
    }));
  }, [typesSelected]);

  async function exportMarkSetGridPdf() {
    setExportingGridPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.markSetGridModel",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          filters: reportFilters,
          studentScope
        },
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
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          filters: reportFilters,
          studentScope
        },
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
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          filters: reportFilters,
          studentScope
        },
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

  async function exportCombinedAnalysisPdf() {
    setExportingCombinedPdf(true);
    props.onError(null);
    try {
      const markSetIds =
        combinedMarkSetIds && combinedMarkSetIds.length > 0
          ? combinedMarkSetIds
          : [props.selectedMarkSetId];
      const model = await requestParsed(
        "reports.combinedAnalysisModel",
        {
          classId: props.selectedClassId,
          markSetIds,
          filters: reportFilters,
          studentScope
        },
        ReportsCombinedAnalysisModelResultSchema
      );
      const html = renderCombinedAnalysisReportHtml(model);
      const setCodes = (model.markSets ?? []).map((m) => m.code).join("-");
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - Combined ${setCodes || "Analysis"}.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingCombinedPdf(false);
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
          studentId: selectedStudentId,
          filters: reportFilters,
          studentScope
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

  async function exportAttendanceMonthlyPdf() {
    setExportingAttendancePdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.attendanceMonthlyModel",
        { classId: props.selectedClassId, month: attendanceMonth },
        ReportsAttendanceMonthlyModelResultSchema
      );
      const html = renderAttendanceMonthlyReportHtml(model as any);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - Attendance ${attendanceMonth}.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingAttendancePdf(false);
    }
  }

  async function exportClassListPdf() {
    setExportingClassListPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.classListModel",
        { classId: props.selectedClassId },
        ReportsClassListModelResultSchema
      );
      const html = renderClassListReportHtml(model as any);
      const defaultFilename = sanitizeFilename(`${model.class.name} - Class List.pdf`);
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingClassListPdf(false);
    }
  }

  async function exportLearningSkillsSummaryPdf() {
    setExportingLearningSkillsPdf(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "reports.learningSkillsSummaryModel",
        { classId: props.selectedClassId, term: learningSkillsTerm },
        ReportsLearningSkillsSummaryModelResultSchema
      );
      const html = renderLearningSkillsSummaryReportHtml(model as any);
      const defaultFilename = sanitizeFilename(
        `${model.class.name} - Learning Skills Term ${learningSkillsTerm}.pdf`
      );
      await window.markbook.exportPdfHtmlWithSaveDialog(html, defaultFilename);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setExportingLearningSkillsPdf(false);
    }
  }

  return (
    <div data-testid="reports-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 8 }}>Reports</div>
      <div
        data-testid="reports-filters-panel"
        style={{
          border: "1px solid #ddd",
          borderRadius: 8,
          padding: 10,
          marginBottom: 14,
          display: "flex",
          flexDirection: "column",
          gap: 8
        }}
      >
        <div style={{ fontWeight: 600 }}>Marks Filters (applies to mark-set reports)</div>
        <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            Scope
            <select
              data-testid="reports-filter-student-scope"
              value={studentScope}
              onChange={(e) =>
                setStudentScope(
                  e.currentTarget.value === "active"
                    ? "active"
                    : e.currentTarget.value === "valid"
                      ? "valid"
                      : "all"
                )
              }
            >
              <option value="all">All students</option>
              <option value="active">Active students</option>
              <option value="valid">Valid for mark set</option>
            </select>
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            Term
            <select
              data-testid="reports-filter-term"
              value={reportFilters.term == null ? "ALL" : String(reportFilters.term)}
              onChange={(e) =>
                setReportFilters((cur) => ({
                  ...cur,
                  term: e.currentTarget.value === "ALL" ? null : Number(e.currentTarget.value)
                }))
              }
            >
              <option value="ALL">ALL</option>
              <option value="1">1</option>
              <option value="2">2</option>
              <option value="3">3</option>
            </select>
          </label>
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            Category
            <select
              data-testid="reports-filter-category"
              value={reportFilters.categoryName ?? "ALL"}
              onChange={(e) =>
                setReportFilters((cur) => ({
                  ...cur,
                  categoryName:
                    e.currentTarget.value === "ALL" ? null : e.currentTarget.value
                }))
              }
            >
              <option value="ALL">ALL</option>
              {categoryOptions.map((c) => (
                <option key={c} value={c}>
                  {c}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div style={{ display: "flex", gap: 10, flexWrap: "wrap" }}>
          {[
            ["Summative", 0],
            ["Formative", 1],
            ["Diagnostic", 2],
            ["Self", 3],
            ["Peer", 4]
          ].map(([label, idx]) => (
            <label key={String(idx)} style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input
                data-testid={`reports-filter-type-${idx}`}
                type="checkbox"
                checked={typesSelected[idx as number]}
                onChange={(e) => {
                  const checked = e.currentTarget.checked;
                  setTypesSelected((cur) => {
                    const next = [...cur] as [boolean, boolean, boolean, boolean, boolean];
                    next[idx as number] = checked;
                    return next;
                  });
                }}
              />
              {label}
            </label>
          ))}
        </div>
      </div>

      <div style={{ color: "#444", marginBottom: 8 }}>Print Mark Set Grid</div>
      <button
        data-testid="export-markset-grid-pdf-btn"
        onClick={() => void exportMarkSetGridPdf()}
        disabled={
          exportingGridPdf ||
          exportingSummaryPdf ||
          exportingCategoryPdf ||
          exportingCombinedPdf ||
          exportingStudentPdf
        }
      >
        {exportingGridPdf ? "Exporting..." : "Export Grid PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Mark Set Summary</div>
      <button
        data-testid="export-markset-summary-pdf-btn"
        onClick={() => void exportMarkSetSummaryPdf()}
        disabled={
          exportingGridPdf ||
          exportingSummaryPdf ||
          exportingCategoryPdf ||
          exportingCombinedPdf ||
          exportingStudentPdf
        }
      >
        {exportingSummaryPdf ? "Exporting..." : "Export Summary PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Category Analysis</div>
      <button
        data-testid="export-category-analysis-pdf-btn"
        onClick={() => void exportCategoryAnalysisPdf()}
        disabled={
          exportingGridPdf ||
          exportingSummaryPdf ||
          exportingCategoryPdf ||
          exportingCombinedPdf ||
          exportingStudentPdf
        }
      >
        {exportingCategoryPdf ? "Exporting..." : "Export Category Analysis PDF"}
      </button>
      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Combined Analysis</div>
      <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
        <button
          data-testid="export-combined-analysis-pdf-btn"
          onClick={() => void exportCombinedAnalysisPdf()}
          disabled={
            exportingGridPdf ||
            exportingSummaryPdf ||
            exportingCategoryPdf ||
            exportingCombinedPdf ||
            exportingStudentPdf
          }
        >
          {exportingCombinedPdf ? "Exporting..." : "Export Combined Analysis PDF"}
        </button>
        <span style={{ color: "#666", fontSize: 12 }}>
          Mark sets:{" "}
          {combinedMarkSetIds && combinedMarkSetIds.length > 0
            ? `${combinedMarkSetIds.length} selected from Combined Analytics`
            : "current mark set only"}
        </span>
      </div>

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
            exportingCombinedPdf ||
            exportingStudentPdf
          }
        >
          {exportingStudentPdf ? "Exporting..." : "Export Student Summary PDF"}
        </button>
      </div>
      <div style={{ marginTop: 12, fontSize: 12, color: "#666" }}>
        Uses Chromium print-to-PDF and preserves legacy mark semantics (blank = No Mark, 0 = Zero).
      </div>

      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Attendance Monthly</div>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <input
          data-testid="attendance-month-input"
          type="month"
          value={attendanceMonth}
          onChange={(e) => setAttendanceMonth(e.currentTarget.value)}
        />
        <button
          data-testid="export-attendance-monthly-pdf-btn"
          onClick={() => void exportAttendanceMonthlyPdf()}
          disabled={exportingAttendancePdf || exportingClassListPdf || exportingLearningSkillsPdf}
        >
          {exportingAttendancePdf ? "Exporting..." : "Export Attendance PDF"}
        </button>
      </div>

      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>Class List</div>
      <button
        data-testid="export-class-list-pdf-btn"
        onClick={() => void exportClassListPdf()}
        disabled={exportingAttendancePdf || exportingClassListPdf || exportingLearningSkillsPdf}
      >
        {exportingClassListPdf ? "Exporting..." : "Export Class List PDF"}
      </button>

      <div style={{ color: "#444", marginTop: 16, marginBottom: 8 }}>
        Learning Skills Summary
      </div>
      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
        <select
          data-testid="learning-skills-term-select"
          value={learningSkillsTerm}
          onChange={(e) => setLearningSkillsTerm(Number(e.currentTarget.value || 1))}
        >
          <option value={1}>Term 1</option>
          <option value={2}>Term 2</option>
          <option value={3}>Term 3</option>
        </select>
        <button
          data-testid="export-learning-skills-pdf-btn"
          onClick={() => void exportLearningSkillsSummaryPdf()}
          disabled={exportingAttendancePdf || exportingClassListPdf || exportingLearningSkillsPdf}
        >
          {exportingLearningSkillsPdf ? "Exporting..." : "Export Learning Skills PDF"}
        </button>
      </div>
    </div>
  );
}
