import React, { useEffect, useMemo, useState } from "react";
import {
  AnalyticsClassAssessmentDrilldownResultSchema,
  AnalyticsClassOpenResultSchema,
  AnalyticsClassRowsResultSchema,
  AnalyticsFiltersOptionsResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type FilterState = {
  term: number | null;
  categoryName: string | null;
  typesMask: number | null;
};

type ClassRowsQueryState = {
  search: string;
  sortBy: "sortOrder" | "displayName" | "finalMark" | "scoredCount" | "zeroCount" | "noMarkCount";
  sortDir: "asc" | "desc";
  page: number;
  pageSize: number;
  cohort: {
    finalMin: number | null;
    finalMax: number | null;
    includeNoFinal: boolean;
  };
};

type DrilldownQueryState = {
  search: string;
  sortBy: "sortOrder" | "displayName" | "status" | "raw" | "percent" | "finalMark";
  sortDir: "asc" | "desc";
  page: number;
  pageSize: number;
};

function formatMark(v: number | null | undefined) {
  if (v == null || !Number.isFinite(v)) return "—";
  return v.toFixed(1);
}

export function ClassAnalyticsScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onOpenReports?: (ctx: {
    filters: FilterState;
    studentScope: "all" | "active" | "valid";
    drilldown?: {
      assessmentId: string;
      query?: {
        search?: string | null;
        sortBy?: "sortOrder" | "displayName" | "status" | "raw" | "percent" | "finalMark";
        sortDir?: "asc" | "desc";
        page?: number;
        pageSize?: number;
      };
    } | null;
  }) => void;
}) {
  const [filters, setFilters] = useState<FilterState>({
    term: null,
    categoryName: null,
    typesMask: null
  });
  const [typesSelected, setTypesSelected] = useState<[boolean, boolean, boolean, boolean, boolean]>(
    [true, true, true, true, true]
  );
  const [studentScope, setStudentScope] = useState<"all" | "active" | "valid">("all");
  const [options, setOptions] = useState<{ terms: number[]; categories: string[] }>({
    terms: [],
    categories: []
  });
  const [summaryLoading, setSummaryLoading] = useState(false);
  const [rowsLoading, setRowsLoading] = useState(false);
  const [drilldownLoading, setDrilldownLoading] = useState(false);
  const [model, setModel] = useState<any>(null);
  const [rowsModel, setRowsModel] = useState<any>(null);
  const [rowsQuery, setRowsQuery] = useState<ClassRowsQueryState>({
    search: "",
    sortBy: "sortOrder",
    sortDir: "asc",
    page: 1,
    pageSize: 25,
    cohort: { finalMin: null, finalMax: null, includeNoFinal: false }
  });
  const [drilldownAssessmentId, setDrilldownAssessmentId] = useState<string | null>(null);
  const [drilldownQuery, setDrilldownQuery] = useState<DrilldownQueryState>({
    search: "",
    sortBy: "sortOrder",
    sortDir: "asc",
    page: 1,
    pageSize: 25
  });
  const [drilldownModel, setDrilldownModel] = useState<any>(null);

  useEffect(() => {
    let mask = 0;
    for (let i = 0; i < typesSelected.length; i += 1) {
      if (typesSelected[i]) mask |= 1 << i;
    }
    setFilters((cur) => ({
      ...cur,
      typesMask: mask === 0 || mask === 31 ? null : mask
    }));
  }, [typesSelected]);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.filters.options",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId
          },
          AnalyticsFiltersOptionsResultSchema
        );
        if (cancelled) return;
        setOptions({
          terms: [...res.terms].sort((a, b) => a - b),
          categories: [...res.categories].sort((a, b) => a.localeCompare(b))
        });
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, props.selectedMarkSetId]);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      setSummaryLoading(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.class.open",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            filters,
            studentScope
          },
          AnalyticsClassOpenResultSchema
        );
        if (cancelled) return;
        setModel(res);
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      } finally {
        if (!cancelled) setSummaryLoading(false);
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [
    props.selectedClassId,
    props.selectedMarkSetId,
    filters.term,
    filters.categoryName,
    filters.typesMask,
    studentScope
  ]);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      setRowsLoading(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.class.rows",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            filters,
            studentScope,
            query: {
              search: rowsQuery.search.trim() ? rowsQuery.search.trim() : null,
              sortBy: rowsQuery.sortBy,
              sortDir: rowsQuery.sortDir,
              page: rowsQuery.page,
              pageSize: rowsQuery.pageSize,
              cohort: rowsQuery.cohort
            }
          },
          AnalyticsClassRowsResultSchema
        );
        if (cancelled) return;
        setRowsModel(res);
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      } finally {
        if (!cancelled) setRowsLoading(false);
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [
    props.selectedClassId,
    props.selectedMarkSetId,
    filters.term,
    filters.categoryName,
    filters.typesMask,
    studentScope,
    rowsQuery.search,
    rowsQuery.sortBy,
    rowsQuery.sortDir,
    rowsQuery.page,
    rowsQuery.pageSize,
    rowsQuery.cohort.finalMin,
    rowsQuery.cohort.finalMax,
    rowsQuery.cohort.includeNoFinal
  ]);

  useEffect(() => {
    if (!drilldownAssessmentId) {
      setDrilldownModel(null);
      return;
    }
    let cancelled = false;
    async function run() {
      setDrilldownLoading(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.class.assessmentDrilldown",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            assessmentId: drilldownAssessmentId,
            filters,
            studentScope,
            query: {
              search: drilldownQuery.search.trim() ? drilldownQuery.search.trim() : null,
              sortBy: drilldownQuery.sortBy,
              sortDir: drilldownQuery.sortDir,
              page: drilldownQuery.page,
              pageSize: drilldownQuery.pageSize
            }
          },
          AnalyticsClassAssessmentDrilldownResultSchema
        );
        if (cancelled) return;
        setDrilldownModel(res);
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      } finally {
        if (!cancelled) setDrilldownLoading(false);
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [
    props.selectedClassId,
    props.selectedMarkSetId,
    drilldownAssessmentId,
    filters.term,
    filters.categoryName,
    filters.typesMask,
    studentScope,
    drilldownQuery.search,
    drilldownQuery.sortBy,
    drilldownQuery.sortDir,
    drilldownQuery.page,
    drilldownQuery.pageSize
  ]);

  const topRows = useMemo(() => model?.topBottom?.top ?? [], [model]);
  const bottomRows = useMemo(() => model?.topBottom?.bottom ?? [], [model]);
  const rows = useMemo(() => rowsModel?.rows ?? [], [rowsModel]);
  const rowsTotal = rowsModel?.totalRows ?? 0;
  const rowsPageCount = Math.max(
    1,
    Math.ceil(rowsTotal / Math.max(1, rowsModel?.pageSize ?? rowsQuery.pageSize))
  );
  const drilldownRows = useMemo(() => drilldownModel?.rows ?? [], [drilldownModel]);
  const drilldownTotal = drilldownModel?.totalRows ?? 0;
  const drilldownPageCount = Math.max(
    1,
    Math.ceil(drilldownTotal / Math.max(1, drilldownModel?.pageSize ?? drilldownQuery.pageSize))
  );

  return (
    <div data-testid="class-analytics-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 10 }}>Class Analytics</div>
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 8,
          padding: 10,
          marginBottom: 14,
          display: "flex",
          gap: 10,
          alignItems: "center",
          flexWrap: "wrap"
        }}
      >
        <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
          Scope
          <select
            data-testid="analytics-filter-scope"
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
            data-testid="analytics-filter-term"
            value={filters.term == null ? "ALL" : String(filters.term)}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setFilters((cur) => ({
                ...cur,
                term: value === "ALL" ? null : Number(value)
              }));
              setRowsQuery((q) => ({ ...q, page: 1 }));
              setDrilldownQuery((q) => ({ ...q, page: 1 }));
            }}
          >
            <option value="ALL">ALL</option>
            {options.terms.map((t) => (
              <option key={t} value={String(t)}>
                {t}
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
          Category
          <select
            data-testid="analytics-filter-category"
            value={filters.categoryName ?? "ALL"}
            onChange={(e) => {
              const value = e.currentTarget.value;
              setFilters((cur) => ({
                ...cur,
                categoryName: value === "ALL" ? null : value
              }));
              setRowsQuery((q) => ({ ...q, page: 1 }));
              setDrilldownQuery((q) => ({ ...q, page: 1 }));
            }}
          >
            <option value="ALL">ALL</option>
            {options.categories.map((c) => (
              <option key={c} value={c}>
                {c}
              </option>
            ))}
          </select>
        </label>

        <div data-testid="analytics-filter-types" style={{ display: "flex", gap: 8 }}>
          {["Sum", "Form", "Diag", "Self", "Peer"].map((label, idx) => (
            <label key={label} style={{ display: "flex", alignItems: "center", gap: 4 }}>
              <input
                type="checkbox"
                checked={typesSelected[idx]}
                onChange={(e) =>
                  setTypesSelected((cur) => {
                    const next = [...cur] as [boolean, boolean, boolean, boolean, boolean];
                    next[idx] = e.currentTarget.checked;
                    return next;
                  })
                }
              />
              {label}
            </label>
          ))}
        </div>

        <button
          data-testid="class-analytics-open-reports"
          onClick={() =>
            props.onOpenReports?.({
              filters,
              studentScope
            })
          }
        >
          Open in Reports
        </button>
      </div>

      {summaryLoading ? <div style={{ color: "#666" }}>Loading analytics…</div> : null}
      {!model ? null : (
        <>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(6, minmax(140px, 1fr))",
              gap: 10,
              marginBottom: 14
            }}
          >
            <div
              data-testid="class-analytics-kpi-average"
              style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}
            >
              <div style={{ color: "#666", fontSize: 12 }}>Class Avg</div>
              <div data-testid="class-analytics-kpi-average-value" style={{ fontWeight: 700 }}>
                {formatMark(model.kpis.classAverage)}
              </div>
            </div>
            <div
              data-testid="class-analytics-kpi-median"
              style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}
            >
              <div style={{ color: "#666", fontSize: 12 }}>Class Median</div>
              <div data-testid="class-analytics-kpi-median-value" style={{ fontWeight: 700 }}>
                {formatMark(model.kpis.classMedian)}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Students</div>
              <div style={{ fontWeight: 700 }}>{model.kpis.studentCount}</div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Final Marks</div>
              <div style={{ fontWeight: 700 }}>{model.kpis.finalMarkCount}</div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>No Mark Rate</div>
              <div style={{ fontWeight: 700 }}>{(model.kpis.noMarkRate * 100).toFixed(1)}%</div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Zero Rate</div>
              <div style={{ fontWeight: 700 }}>{(model.kpis.zeroRate * 100).toFixed(1)}%</div>
            </div>
          </div>

          <div style={{ marginBottom: 14, display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
            <span style={{ fontWeight: 600 }}>Histogram Cohort:</span>
            {(model.distributions?.bins ?? []).map((bin: any) => (
              <button
                key={bin.label}
                data-testid={`analytics-class-bin-filter-${bin.label}`}
                onClick={() =>
                  setRowsQuery((q) => ({
                    ...q,
                    page: 1,
                    cohort: {
                      finalMin: Number(bin.min),
                      finalMax: Number(bin.max),
                      includeNoFinal: false
                    }
                  }))
                }
              >
                {bin.label} ({bin.count})
              </button>
            ))}
            <button
              onClick={() =>
                setRowsQuery((q) => ({
                  ...q,
                  page: 1,
                  cohort: { finalMin: null, finalMax: null, includeNoFinal: false }
                }))
              }
            >
              Clear Cohort
            </button>
          </div>

          <div style={{ display: "flex", gap: 16, marginBottom: 14 }}>
            <div style={{ flex: 1, border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Top Students</div>
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {topRows.map((row: any) => (
                  <li key={`top-${row.studentId}`}>
                    {row.displayName} ({formatMark(row.finalMark)})
                  </li>
                ))}
              </ul>
            </div>
            <div style={{ flex: 1, border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Bottom Students</div>
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {bottomRows.map((row: any) => (
                  <li key={`bot-${row.studentId}`}>
                    {row.displayName} ({formatMark(row.finalMark)})
                  </li>
                ))}
              </ul>
            </div>
          </div>

          <div style={{ display: "flex", gap: 16 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Per Assessment</div>
              <div style={{ maxHeight: 300, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Assessment</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Avg Raw</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Avg %</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Median %</th>
                      <th style={{ textAlign: "right", padding: 6 }} />
                    </tr>
                  </thead>
                  <tbody>
                    {(model.perAssessment ?? []).map((a: any) => (
                      <tr key={a.assessmentId}>
                        <td style={{ padding: 6 }}>{a.title}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.avgRaw.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.avgPercent.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.medianPercent.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>
                          <button
                            data-testid={`analytics-assessment-drilldown-open-${a.assessmentId}`}
                            onClick={() => {
                              setDrilldownAssessmentId(a.assessmentId);
                              setDrilldownQuery((q) => ({ ...q, page: 1 }));
                            }}
                          >
                            Drilldown
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
            <div style={{ width: 340 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Per Category</div>
              <div style={{ maxHeight: 300, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Category</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Weight</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Class Avg</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(model.perCategory ?? []).map((c: any) => (
                      <tr key={c.name}>
                        <td style={{ padding: 6 }}>{c.name}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{c.weight.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{c.classAvg.toFixed(1)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          <div style={{ marginTop: 16 }}>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>Student Rows</div>
            <div
              style={{
                border: "1px solid #ddd",
                borderRadius: 8,
                padding: 10,
                marginBottom: 8,
                display: "flex",
                gap: 8,
                alignItems: "center",
                flexWrap: "wrap"
              }}
            >
              <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
                Search
                <input
                  data-testid="analytics-class-search"
                  value={rowsQuery.search}
                  onChange={(e) =>
                    setRowsQuery((q) => ({ ...q, search: e.currentTarget.value, page: 1 }))
                  }
                />
              </label>
              <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
                Sort
                <select
                  data-testid="analytics-class-sort"
                  value={`${rowsQuery.sortBy}:${rowsQuery.sortDir}`}
                  onChange={(e) => {
                    const [sortBy, sortDir] = e.currentTarget.value.split(":");
                    setRowsQuery((q) => ({
                      ...q,
                      sortBy: sortBy as ClassRowsQueryState["sortBy"],
                      sortDir: (sortDir as "asc" | "desc") || "asc",
                      page: 1
                    }));
                  }}
                >
                  <option value="sortOrder:asc">Sort Order ↑</option>
                  <option value="sortOrder:desc">Sort Order ↓</option>
                  <option value="displayName:asc">Name ↑</option>
                  <option value="displayName:desc">Name ↓</option>
                  <option value="finalMark:desc">Final Mark ↓</option>
                  <option value="finalMark:asc">Final Mark ↑</option>
                </select>
              </label>
              <span style={{ color: "#666", fontSize: 12 }}>
                {rowsLoading
                  ? "Loading rows..."
                  : `${rowsTotal} total rows, page ${rowsModel?.page ?? 1}/${rowsPageCount}`}
              </span>
            </div>

            <div style={{ maxHeight: 260, overflow: "auto", border: "1px solid #ddd" }}>
              <table style={{ width: "100%", borderCollapse: "collapse" }}>
                <thead>
                  <tr>
                    <th style={{ textAlign: "left", padding: 6 }}>Student</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Final</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Scored</th>
                    <th style={{ textAlign: "right", padding: 6 }}>Zero</th>
                    <th style={{ textAlign: "right", padding: 6 }}>No Mark</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r: any) => (
                    <tr key={r.studentId}>
                      <td style={{ padding: 6 }}>{r.displayName}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{formatMark(r.finalMark)}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{r.scoredCount}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{r.zeroCount}</td>
                      <td style={{ padding: 6, textAlign: "right" }}>{r.noMarkCount}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
              <button
                onClick={() =>
                  setRowsQuery((q) => ({ ...q, page: Math.max(1, q.page - 1) }))
                }
                disabled={(rowsModel?.page ?? 1) <= 1}
              >
                Prev
              </button>
              <button
                data-testid="analytics-class-page-next"
                onClick={() =>
                  setRowsQuery((q) => ({
                    ...q,
                    page: Math.min(rowsPageCount, q.page + 1)
                  }))
                }
                disabled={(rowsModel?.page ?? 1) >= rowsPageCount}
              >
                Next
              </button>
            </div>
          </div>

          {drilldownAssessmentId ? (
            <div
              data-testid="analytics-assessment-drilldown-panel"
              style={{
                marginTop: 18,
                border: "1px solid #bbb",
                borderRadius: 10,
                padding: 12
              }}
            >
              <div style={{ fontWeight: 700, marginBottom: 8 }}>
                Assessment Drilldown {drilldownModel?.assessment?.title ? `- ${drilldownModel.assessment.title}` : ""}
              </div>
              <div style={{ display: "flex", gap: 8, flexWrap: "wrap", alignItems: "center", marginBottom: 8 }}>
                <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  Search
                  <input
                    value={drilldownQuery.search}
                    onChange={(e) =>
                      setDrilldownQuery((q) => ({ ...q, search: e.currentTarget.value, page: 1 }))
                    }
                  />
                </label>
                <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
                  Sort
                  <select
                    value={`${drilldownQuery.sortBy}:${drilldownQuery.sortDir}`}
                    onChange={(e) => {
                      const [sortBy, sortDir] = e.currentTarget.value.split(":");
                      setDrilldownQuery((q) => ({
                        ...q,
                        sortBy: sortBy as DrilldownQueryState["sortBy"],
                        sortDir: (sortDir as "asc" | "desc") || "asc",
                        page: 1
                      }));
                    }}
                  >
                    <option value="sortOrder:asc">Sort Order ↑</option>
                    <option value="displayName:asc">Name ↑</option>
                    <option value="status:asc">Status ↑</option>
                    <option value="percent:desc">Percent ↓</option>
                    <option value="finalMark:desc">Final Mark ↓</option>
                  </select>
                </label>
                <button
                  onClick={() =>
                    props.onOpenReports?.({
                      filters,
                      studentScope,
                      drilldown: {
                        assessmentId: drilldownAssessmentId,
                        query: {
                          search: drilldownQuery.search.trim() || null,
                          sortBy: drilldownQuery.sortBy,
                          sortDir: drilldownQuery.sortDir,
                          page: drilldownQuery.page,
                          pageSize: drilldownQuery.pageSize
                        }
                      }
                    })
                  }
                >
                  Open Drilldown in Reports
                </button>
                <span style={{ color: "#666", fontSize: 12 }}>
                  {drilldownLoading
                    ? "Loading drilldown..."
                    : `${drilldownTotal} rows, page ${drilldownModel?.page ?? 1}/${drilldownPageCount}`}
                </span>
              </div>

              <div style={{ marginBottom: 8, color: "#444", fontSize: 12 }}>
                Class stats: raw {formatMark(drilldownModel?.classStats?.avgRaw)} /{" "}
                {formatMark(drilldownModel?.classStats?.avgPercent)}%
              </div>

              <div style={{ maxHeight: 260, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Student</th>
                      <th style={{ textAlign: "left", padding: 6 }}>Status</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Raw</th>
                      <th style={{ textAlign: "right", padding: 6 }}>%</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Final</th>
                    </tr>
                  </thead>
                  <tbody>
                    {drilldownRows.map((r: any) => (
                      <tr key={r.studentId}>
                        <td style={{ padding: 6 }}>{r.displayName}</td>
                        <td style={{ padding: 6 }}>{r.status}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(r.raw)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(r.percent)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(r.finalMark)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
              <div style={{ marginTop: 8, display: "flex", gap: 8 }}>
                <button
                  onClick={() =>
                    setDrilldownQuery((q) => ({ ...q, page: Math.max(1, q.page - 1) }))
                  }
                  disabled={(drilldownModel?.page ?? 1) <= 1}
                >
                  Prev
                </button>
                <button
                  onClick={() =>
                    setDrilldownQuery((q) => ({
                      ...q,
                      page: Math.min(drilldownPageCount, q.page + 1)
                    }))
                  }
                  disabled={(drilldownModel?.page ?? 1) >= drilldownPageCount}
                >
                  Next
                </button>
              </div>
            </div>
          ) : null}
        </>
      )}
    </div>
  );
}
