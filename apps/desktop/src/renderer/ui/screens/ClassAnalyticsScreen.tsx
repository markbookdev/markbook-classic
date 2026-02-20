import React, { useEffect, useMemo, useState } from "react";
import {
  AnalyticsClassOpenResultSchema,
  AnalyticsFiltersOptionsResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type FilterState = {
  term: number | null;
  categoryName: string | null;
  typesMask: number | null;
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
  const [options, setOptions] = useState<{
    terms: number[];
    categories: string[];
  }>({ terms: [], categories: [] });
  const [loading, setLoading] = useState(false);
  const [model, setModel] = useState<
    | {
        filters: FilterState;
        studentScope: "all" | "active" | "valid";
        kpis: {
          classAverage: number | null;
          classMedian: number | null;
          studentCount: number;
          finalMarkCount: number;
          noMarkRate: number;
          zeroRate: number;
        };
        distributions: {
          bins: Array<{ label: string; min: number; max: number; count: number }>;
          noFinalMarkCount: number;
        };
        perAssessment: Array<{
          assessmentId: string;
          idx: number;
          title: string;
          outOf: number;
          avgRaw: number;
          avgPercent: number;
          medianPercent: number;
          scoredCount: number;
          zeroCount: number;
          noMarkCount: number;
        }>;
        perCategory: Array<{
          name: string;
          weight: number;
          classAvg: number;
          studentCount: number;
          assessmentCount: number;
        }>;
        topBottom: {
          top: Array<{ studentId: string; displayName: string; finalMark: number | null }>;
          bottom: Array<{ studentId: string; displayName: string; finalMark: number | null }>;
        };
        rows: Array<{
          studentId: string;
          displayName: string;
          finalMark: number | null;
          noMarkCount: number;
          zeroCount: number;
          scoredCount: number;
        }>;
      }
    | null
  >(null);

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
      setLoading(true);
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
        setModel(res as any);
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, props.selectedMarkSetId, filters.term, filters.categoryName, filters.typesMask, studentScope]);

  const topRows = useMemo(() => model?.topBottom.top ?? [], [model]);
  const bottomRows = useMemo(() => model?.topBottom.bottom ?? [], [model]);

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

      {loading ? <div style={{ color: "#666" }}>Loading analytics…</div> : null}
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
            <div data-testid="class-analytics-kpi-average" style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Class Avg</div>
              <div data-testid="class-analytics-kpi-average-value" style={{ fontWeight: 700 }}>
                {formatMark(model.kpis.classAverage)}
              </div>
            </div>
            <div data-testid="class-analytics-kpi-median" style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
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

          <div style={{ display: "flex", gap: 16, marginBottom: 14 }}>
            <div style={{ flex: 1, border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Top Students</div>
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {topRows.map((row) => (
                  <li key={`top-${row.studentId}`}>
                    {row.displayName} ({formatMark(row.finalMark)})
                  </li>
                ))}
              </ul>
            </div>
            <div style={{ flex: 1, border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Bottom Students</div>
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {bottomRows.map((row) => (
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
                    </tr>
                  </thead>
                  <tbody>
                    {model.perAssessment.map((a) => (
                      <tr key={a.assessmentId}>
                        <td style={{ padding: 6 }}>{a.title}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.avgRaw.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.avgPercent.toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{a.medianPercent.toFixed(1)}</td>
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
                    {model.perCategory.map((c) => (
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
        </>
      )}
    </div>
  );
}
