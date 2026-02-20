import React, { useEffect, useMemo, useState } from "react";
import {
  AnalyticsFiltersOptionsResultSchema,
  AnalyticsStudentOpenResultSchema,
  MarkSetOpenResultSchema
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

export function StudentAnalyticsScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onOpenReports?: (ctx: {
    filters: FilterState;
    studentScope: "all" | "active" | "valid";
    studentId?: string | null;
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
  const [students, setStudents] = useState<Array<{ id: string; displayName: string }>>([]);
  const [selectedStudentId, setSelectedStudentId] = useState<string | null>(null);
  const [options, setOptions] = useState<{
    terms: number[];
    categories: string[];
  }>({ terms: [], categories: [] });
  const [loading, setLoading] = useState(false);
  const [model, setModel] = useState<any>(null);

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
        const [opts, open] = await Promise.all([
          requestParsed(
            "analytics.filters.options",
            {
              classId: props.selectedClassId,
              markSetId: props.selectedMarkSetId
            },
            AnalyticsFiltersOptionsResultSchema
          ),
          requestParsed(
            "markset.open",
            {
              classId: props.selectedClassId,
              markSetId: props.selectedMarkSetId
            },
            MarkSetOpenResultSchema
          )
        ]);
        if (cancelled) return;
        setOptions({
          terms: [...opts.terms].sort((a, b) => a - b),
          categories: [...opts.categories].sort((a, b) => a.localeCompare(b))
        });
        const roster = open.students.map((s) => ({ id: s.id, displayName: s.displayName }));
        setStudents(roster);
        setSelectedStudentId((cur) => {
          if (cur && roster.some((s) => s.id === cur)) return cur;
          return roster[0]?.id ?? null;
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
    if (!selectedStudentId) {
      setModel(null);
      return;
    }
    let cancelled = false;
    async function run() {
      setLoading(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "analytics.student.open",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            studentId: selectedStudentId,
            filters
          },
          AnalyticsStudentOpenResultSchema
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
  }, [props.selectedClassId, props.selectedMarkSetId, selectedStudentId, filters.term, filters.categoryName, filters.typesMask]);

  const categoryRows = useMemo(() => model?.categoryBreakdown ?? [], [model]);
  const assessmentRows = useMemo(() => model?.assessmentTrail ?? [], [model]);

  return (
    <div data-testid="student-analytics-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 700, marginBottom: 10 }}>Student Analytics</div>
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
          Student
          <select
            data-testid="analytics-student-select"
            value={selectedStudentId ?? ""}
            onChange={(e) => setSelectedStudentId(e.currentTarget.value || null)}
          >
            {students.map((s) => (
              <option key={s.id} value={s.id}>
                {s.displayName}
              </option>
            ))}
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
          data-testid="student-analytics-open-reports"
          onClick={() =>
            props.onOpenReports?.({
              filters,
              studentScope: "valid",
              studentId: selectedStudentId
            })
          }
        >
          Open in Reports
        </button>
      </div>

      {loading ? <div style={{ color: "#666" }}>Loading analytics…</div> : null}
      {!model ? null : (
        <>
          <div style={{ display: "flex", gap: 14, marginBottom: 14 }}>
            <div data-testid="student-analytics-final-mark" style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Final Mark</div>
              <div data-testid="student-analytics-final-mark-value" style={{ fontWeight: 700, fontSize: 18 }}>
                {formatMark(model.finalMark)}
              </div>
            </div>
            <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
              <div style={{ color: "#666", fontSize: 12 }}>Scored / Zero / No Mark</div>
              <div style={{ fontWeight: 700 }}>
                {model.counts.scored} / {model.counts.zero} / {model.counts.noMark}
              </div>
            </div>
            {model.attendanceSummary ? (
              <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 10 }}>
                <div style={{ color: "#666", fontSize: 12 }}>Attendance Summary</div>
                <div style={{ fontWeight: 700 }}>
                  {model.attendanceSummary.monthsWithData} mo, {model.attendanceSummary.codedDays} coded days
                </div>
              </div>
            ) : null}
          </div>

          <div style={{ display: "flex", gap: 16 }}>
            <div style={{ width: 320 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Category Breakdown</div>
              <div style={{ maxHeight: 320, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Category</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Weight</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Value</th>
                    </tr>
                  </thead>
                  <tbody>
                    {categoryRows.map((c: any) => (
                      <tr key={c.name}>
                        <td style={{ padding: 6 }}>{c.name}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{Number(c.weight).toFixed(1)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(c.value)}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>

            <div style={{ flex: 1, minWidth: 0 }}>
              <div style={{ fontWeight: 600, marginBottom: 6 }}>Assessment Trail</div>
              <div style={{ maxHeight: 420, overflow: "auto", border: "1px solid #ddd" }}>
                <table style={{ width: "100%", borderCollapse: "collapse" }}>
                  <thead>
                    <tr>
                      <th style={{ textAlign: "left", padding: 6 }}>Assessment</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Score</th>
                      <th style={{ textAlign: "right", padding: 6 }}>%</th>
                      <th style={{ textAlign: "right", padding: 6 }}>Class Avg %</th>
                    </tr>
                  </thead>
                  <tbody>
                    {assessmentRows.map((a: any) => (
                      <tr key={a.assessmentId}>
                        <td style={{ padding: 6 }}>{a.title}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(a.score)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(a.percent)}</td>
                        <td style={{ padding: 6, textAlign: "right" }}>{formatMark(a.classAvgPercent)}</td>
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
