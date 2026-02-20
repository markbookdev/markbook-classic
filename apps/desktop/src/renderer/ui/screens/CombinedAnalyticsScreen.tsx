import React, { useEffect, useMemo, useState } from "react";
import {
  AnalyticsCombinedOpenResultSchema,
  AnalyticsCombinedOptionsResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type FilterState = {
  term: number | null;
  categoryName: string | null;
  typesMask: number | null;
};

type MarkSetOption = {
  id: string;
  code: string;
  description: string;
  sortOrder: number;
  weight: number;
  deletedAt?: string | null;
};

function formatMark(v: number | null | undefined) {
  if (v == null || !Number.isFinite(v)) return "—";
  return v.toFixed(1);
}

export function CombinedAnalyticsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
  onOpenReports?: (ctx: {
    filters: FilterState;
    studentScope: "all" | "active" | "valid";
    markSetIds: string[];
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
    markSets: MarkSetOption[];
    terms: number[];
    categories: string[];
  }>({ markSets: [], terms: [], categories: [] });
  const [selectedMarkSetIds, setSelectedMarkSetIds] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const [model, setModel] = useState<any>(null);

  const selectedCodes = useMemo(() => {
    const byId = new Map(options.markSets.map((m) => [m.id, m.code]));
    return selectedMarkSetIds.map((id) => byId.get(id) ?? id).join(", ");
  }, [options.markSets, selectedMarkSetIds]);

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
          "analytics.combined.options",
          { classId: props.selectedClassId },
          AnalyticsCombinedOptionsResultSchema
        );
        if (cancelled) return;
        const markSets = [...res.markSets]
          .filter((m) => m.deletedAt == null)
          .sort((a, b) => a.sortOrder - b.sortOrder) as MarkSetOption[];
        setOptions({
          markSets,
          terms: [...res.terms].sort((a, b) => a - b),
          categories: [...res.categories].sort((a, b) => a.localeCompare(b))
        });
        setSelectedMarkSetIds((cur) => {
          const valid = cur.filter((id) => markSets.some((m) => m.id === id));
          return valid.length > 0 ? valid : markSets.map((m) => m.id);
        });
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
        setOptions({ markSets: [], terms: [], categories: [] });
        setSelectedMarkSetIds([]);
      }
    }
    void run();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId]);

  useEffect(() => {
    if (selectedMarkSetIds.length === 0) {
      setModel(null);
      return;
    }
    let cancelled = false;
    async function run() {
      setLoading(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.combined.open",
          {
            classId: props.selectedClassId,
            markSetIds: selectedMarkSetIds,
            filters,
            studentScope
          },
          AnalyticsCombinedOpenResultSchema
        );
        if (cancelled) return;
        setModel(res);
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
  }, [
    props.selectedClassId,
    selectedMarkSetIds.join(","),
    filters.term,
    filters.categoryName,
    filters.typesMask,
    studentScope
  ]);

  return (
    <div data-testid="combined-analytics-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 10 }}>Combined Analytics</div>
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
        <label style={{ display: "flex", flexDirection: "column", gap: 6, minWidth: 280 }}>
          Mark Sets
          <select
            multiple
            size={Math.min(6, Math.max(3, options.markSets.length))}
            data-testid="combined-analytics-markset-multiselect"
            value={selectedMarkSetIds}
            onChange={(e) => {
              const ids = Array.from(e.currentTarget.selectedOptions).map((o) => o.value);
              setSelectedMarkSetIds(ids);
            }}
          >
            {options.markSets.map((m) => (
              <option key={m.id} value={m.id}>
                {m.code} ({m.weight.toFixed(1)})
              </option>
            ))}
          </select>
        </label>

        <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
          Scope
          <select
            data-testid="combined-analytics-filter-scope"
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
            <option value="valid">Valid in selected sets</option>
          </select>
        </label>

        <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
          Term
          <select
            data-testid="combined-analytics-filter-term"
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
            data-testid="combined-analytics-filter-category"
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

        <div data-testid="combined-analytics-filter-types" style={{ display: "flex", gap: 8 }}>
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
          data-testid="combined-analytics-open-reports"
          disabled={selectedMarkSetIds.length === 0}
          onClick={() =>
            props.onOpenReports?.({
              filters,
              studentScope,
              markSetIds: selectedMarkSetIds
            })
          }
        >
          Open in Reports
        </button>
      </div>

      <div style={{ color: "#666", marginBottom: 8 }}>
        Selected mark sets: {selectedCodes || "(none)"}
      </div>

      {loading ? <div style={{ color: "#666" }}>Loading analytics…</div> : null}
      {!model ? null : (
        <>
          <div
            style={{
              display: "grid",
              gridTemplateColumns: "repeat(5, minmax(140px, 1fr))",
              gap: 10,
              marginBottom: 14
            }}
          >
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Combined Avg</div>
              <div data-testid="combined-analytics-kpi-average-value" style={{ fontWeight: 700 }}>
                {formatMark(model.kpis.classAverage)}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Combined Median</div>
              <div data-testid="combined-analytics-kpi-median-value" style={{ fontWeight: 700 }}>
                {formatMark(model.kpis.classMedian)}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Students</div>
              <div data-testid="combined-analytics-kpi-student-count" style={{ fontWeight: 700 }}>
                {model.kpis.studentCount}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Finals</div>
              <div data-testid="combined-analytics-kpi-final-count" style={{ fontWeight: 700 }}>
                {model.kpis.finalMarkCount}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>No Combined Final</div>
              <div data-testid="combined-analytics-kpi-no-final-count" style={{ fontWeight: 700 }}>
                {model.kpis.noCombinedFinalCount}
              </div>
            </div>
          </div>

          <div style={{ display: "flex", gap: 14, marginBottom: 14 }}>
            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Per Mark Set</div>
              <div style={{ maxHeight: 220, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Mark Set</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Weight</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Avg</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Median</th>
                    </tr>
                  </thead>
                  <tbody>
                    {(model.perMarkSet ?? []).map((m: any) => (
                      <tr key={m.markSetId}>
                        <td style={{ padding: 6 }}>
                          {m.code}: {m.description}
                        </td>
                        <td style={{ padding: 6, textAlign: "right" }}>{Number(m.weight).toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(m.classAverage)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(m.classMedian)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
            <div style={{ width: 300 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Top / Bottom</div>
              <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 8 }}>
                <div style={{ fontSize: 12, color: "#666", marginBottom: 4 }}>Top</div>
                {(model.topBottom?.top ?? []).map((s: any) => (
                  <div key={`top-${s.studentId}`} style={{ display: "flex", justifyContent: "space-between" }}>
                    <span>{s.displayName}</span>
                    <strong>{formatMark(s.combinedFinal)}</strong>
                  </div>
                ))}
                <div style={{ fontSize: 12, color: "#666", margin: "8px 0 4px" }}>Bottom</div>
                {(model.topBottom?.bottom ?? []).map((s: any) => (
                  <div key={`bot-${s.studentId}`} style={{ display: "flex", justifyContent: "space-between" }}>
                    <span>{s.displayName}</span>
                    <strong>{formatMark(s.combinedFinal)}</strong>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <div style={{ border: "1px solid #ddd", maxHeight: 360, overflow: "auto" }}>
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr>
                  <th style={{ textAlign: "left", padding: 6, position: "sticky", top: 0, background: "#f7f7f7" }}>
                    Student
                  </th>
                  <th style={{ textAlign: "right", padding: 6, position: "sticky", top: 0, background: "#f7f7f7" }}>
                    Combined
                  </th>
                  {model.markSets.map((m: any) => (
                    <th
                      key={`head-${m.id}`}
                      style={{ textAlign: "right", padding: 6, position: "sticky", top: 0, background: "#f7f7f7" }}
                    >
                      {m.code}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {(model.rows ?? []).map((r: any) => {
                  const perMap = new Map(
                    (r.perMarkSet ?? []).map((x: any) => [x.markSetId, x.finalMark] as const)
                  );
                  return (
                    <tr key={r.studentId}>
                      <td style={{ padding: 6 }}>{r.displayName}</td>
                      <td style={{ padding: 6, textAlign: "right", fontWeight: 700 }}>
                        {formatMark(r.combinedFinal)}
                      </td>
                      {model.markSets.map((m: any) => (
                        <td key={`${r.studentId}-${m.id}`} style={{ padding: 6, textAlign: "right" }}>
                          {formatMark(perMap.get(m.id))}
                        </td>
                      ))}
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </>
      )}
    </div>
  );
}
