import React, { useEffect, useMemo, useState } from "react";
import {
  AssessmentsCreateResultSchema,
  AssessmentsDeleteResultSchema,
  AssessmentsListResultSchema,
  AssessmentsReorderResultSchema,
  AssessmentsUpdateResultSchema,
  CategoriesCreateResultSchema,
  CategoriesDeleteResultSchema,
  CategoriesListResultSchema,
  CategoriesUpdateResultSchema,
  MarkSetSettingsGetResultSchema,
  MarkSetSettingsUpdateResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type CategoryRow = {
  id: string;
  name: string;
  weight: number | null;
  sortOrder: number;
};

type AssessmentRow = {
  id: string;
  idx: number;
  date: string | null;
  categoryName: string | null;
  title: string;
  term: number | null;
  legacyType: number | null;
  weight: number | null;
  outOf: number | null;
};

function parseNullableNumber(s: string): number | null {
  const t = s.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return n;
}

function parseNullableInt(s: string): number | null {
  const t = s.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return Math.trunc(n);
}

export function MarkSetSetupScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onChanged?: () => void | Promise<void>;
}) {
  const [loading, setLoading] = useState(false);
  const [categories, setCategories] = useState<CategoryRow[]>([]);
  const [assessments, setAssessments] = useState<AssessmentRow[]>([]);
  const [fullCode, setFullCode] = useState("");
  const [room, setRoom] = useState("");
  const [day, setDay] = useState("");
  const [period, setPeriod] = useState("");
  const [weightMethod, setWeightMethod] = useState("1");
  const [calcMethod, setCalcMethod] = useState("0");

  const [newCategoryName, setNewCategoryName] = useState("");
  const [newCategoryWeight, setNewCategoryWeight] = useState("20");

  const [newTitle, setNewTitle] = useState("");
  const [newDate, setNewDate] = useState("");
  const [newCategoryName2, setNewCategoryName2] = useState("");
  const [newTerm, setNewTerm] = useState("1");
  const [newWeight, setNewWeight] = useState("1");
  const [newOutOf, setNewOutOf] = useState("10");

  const canAddCategory = useMemo(() => newCategoryName.trim().length > 0, [newCategoryName]);
  const canAddAssessment = useMemo(() => newTitle.trim().length > 0, [newTitle]);

  async function loadAll() {
    setLoading(true);
    props.onError(null);
    try {
      const [cats, asmt, settings] = await Promise.all([
        requestParsed(
          "categories.list",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          CategoriesListResultSchema
        ),
        requestParsed(
          "assessments.list",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          AssessmentsListResultSchema
        ),
        requestParsed(
          "markset.settings.get",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          MarkSetSettingsGetResultSchema
        )
      ]);
      setCategories(cats.categories);
      setAssessments(asmt.assessments);
      setFullCode(settings.markSet.fullCode ?? "");
      setRoom(settings.markSet.room ?? "");
      setDay(settings.markSet.day ?? "");
      setPeriod(settings.markSet.period ?? "");
      setWeightMethod(String(settings.markSet.weightMethod ?? 1));
      setCalcMethod(String(settings.markSet.calcMethod ?? 0));
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setCategories([]);
      setAssessments([]);
      setFullCode("");
      setRoom("");
      setDay("");
      setPeriod("");
      setWeightMethod("1");
      setCalcMethod("0");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, props.selectedMarkSetId]);

  async function saveMarkSetSettings() {
    props.onError(null);
    try {
      await requestParsed(
        "markset.settings.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          patch: {
            fullCode: fullCode.trim() || null,
            room: room.trim() || null,
            day: day.trim() || null,
            period: period.trim() || null,
            weightMethod: parseNullableInt(weightMethod) ?? 1,
            calcMethod: parseNullableInt(calcMethod) ?? 0
          }
        },
        MarkSetSettingsUpdateResultSchema
      );
      await props.onChanged?.();
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function updateCategoryLocal(id: string, patch: Partial<CategoryRow>) {
    setCategories((prev) => prev.map((c) => (c.id === id ? { ...c, ...patch } : c)));
  }
  async function updateCategory(
    categoryId: string,
    patch: { name?: string; weight?: number | null }
  ) {
    props.onError(null);
    try {
      await requestParsed(
        "categories.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          categoryId,
          patch
        },
        CategoriesUpdateResultSchema
      );
      updateCategoryLocal(categoryId, patch as any);
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  async function addCategory() {
    if (!canAddCategory) return;
    props.onError(null);
    try {
      await requestParsed(
        "categories.create",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          name: newCategoryName.trim(),
          weight: parseNullableNumber(newCategoryWeight) ?? 0
        },
        CategoriesCreateResultSchema
      );
      setNewCategoryName("");
      setNewCategoryWeight("20");
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteCategory(categoryId: string) {
    const ok = confirm("Delete this category?");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "categories.delete",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          categoryId
        },
        CategoriesDeleteResultSchema
      );
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function updateAssessmentLocal(id: string, patch: Partial<AssessmentRow>) {
    setAssessments((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }

  async function updateAssessment(
    assessmentId: string,
    patch: {
      date?: string | null;
      categoryName?: string | null;
      title?: string;
      term?: number | null;
      legacyType?: number | null;
      weight?: number | null;
      outOf?: number | null;
    }
  ) {
    props.onError(null);
    try {
      await requestParsed(
        "assessments.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          assessmentId,
          patch
        },
        AssessmentsUpdateResultSchema
      );
      updateAssessmentLocal(assessmentId, patch as any);
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  async function addAssessment() {
    if (!canAddAssessment) return;
    props.onError(null);
    try {
      await requestParsed(
        "assessments.create",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          title: newTitle.trim(),
          date: newDate.trim() ? newDate.trim() : null,
          categoryName: newCategoryName2.trim() ? newCategoryName2.trim() : null,
          term: parseNullableInt(newTerm),
          weight: parseNullableNumber(newWeight),
          outOf: parseNullableNumber(newOutOf)
        },
        AssessmentsCreateResultSchema
      );
      setNewTitle("");
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteAssessment(assessmentId: string) {
    const ok = confirm("Delete this assessment? All related marks will be removed.");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "assessments.delete",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          assessmentId
        },
        AssessmentsDeleteResultSchema
      );
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function moveAssessment(idx: number, dir: -1 | 1) {
    const nextIdx = idx + dir;
    if (nextIdx < 0 || nextIdx >= assessments.length) return;
    const next = assessments.slice();
    const [row] = next.splice(idx, 1);
    next.splice(nextIdx, 0, row);
    const orderedAssessmentIds = next.map((a) => a.id);

    props.onError(null);
    try {
      await requestParsed(
        "assessments.reorder",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          orderedAssessmentIds
        },
        AssessmentsReorderResultSchema
      );
      setAssessments(next.map((a, i) => ({ ...a, idx: i })));
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  const inputStyle: React.CSSProperties = useMemo(
    () => ({
      width: "100%",
      padding: "6px 8px",
      border: "1px solid #ddd",
      borderRadius: 6
    }),
    []
  );

  return (
    <div data-testid="markset-setup-screen" style={{ padding: 24, maxWidth: 1200 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Mark Set Setup</div>

      <div style={{ display: "flex", gap: 16, alignItems: "center", marginBottom: 10 }}>
        <div style={{ color: "#555", fontSize: 13 }}>
          {loading
            ? "Loading..."
            : `${categories.length} categories, ${assessments.length} assessments`}
        </div>
        <button onClick={() => void loadAll()} disabled={loading}>
          Reload
        </button>
      </div>

      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          marginBottom: 16
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Mark Set Settings</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            data-testid="markset-fullcode-input"
            value={fullCode}
            onChange={(e) => setFullCode(e.currentTarget.value)}
            placeholder="Full code"
            style={{ ...inputStyle, flex: "2 1 220px" }}
          />
          <input
            data-testid="markset-room-input"
            value={room}
            onChange={(e) => setRoom(e.currentTarget.value)}
            placeholder="Room"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <input
            data-testid="markset-day-input"
            value={day}
            onChange={(e) => setDay(e.currentTarget.value)}
            placeholder="Day"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <input
            data-testid="markset-period-input"
            value={period}
            onChange={(e) => setPeriod(e.currentTarget.value)}
            placeholder="Period"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <select
            data-testid="markset-weightmethod-select"
            value={weightMethod}
            onChange={(e) => setWeightMethod(e.currentTarget.value)}
            style={{ ...inputStyle, flex: "1 1 180px" }}
          >
            <option value="0">Weighting: Entry</option>
            <option value="1">Weighting: Category</option>
            <option value="2">Weighting: Equal</option>
          </select>
          <select
            data-testid="markset-calcmethod-select"
            value={calcMethod}
            onChange={(e) => setCalcMethod(e.currentTarget.value)}
            style={{ ...inputStyle, flex: "1 1 180px" }}
          >
            <option value="0">Calc Method 0</option>
            <option value="1">Calc Method 1</option>
            <option value="2">Calc Method 2</option>
            <option value="3">Calc Method 3</option>
            <option value="4">Calc Method 4</option>
          </select>
          <button data-testid="markset-save-settings-btn" onClick={() => void saveMarkSetSettings()}>
            Save Settings
          </button>
        </div>
      </div>

      <div style={{ display: "flex", gap: 16, minHeight: 0 }}>
        <div
          style={{
            flex: "0 0 360px",
            border: "1px solid #ddd",
            borderRadius: 10,
            padding: 16,
            height: "calc(100vh - 220px)",
            overflow: "auto"
          }}
        >
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Categories</div>

          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.currentTarget.value)}
              placeholder="Name"
              style={{ ...inputStyle, flex: 1 }}
            />
            <input
              value={newCategoryWeight}
              onChange={(e) => setNewCategoryWeight(e.currentTarget.value)}
              placeholder="Weight"
              style={{ ...inputStyle, width: 90 }}
            />
            <button disabled={!canAddCategory} onClick={() => void addCategory()}>
              Add
            </button>
          </div>

          {categories.length === 0 ? (
            <div style={{ color: "#666" }}>(none yet)</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {categories.map((c) => (
                <div
                  key={c.id}
                  style={{
                    border: "1px solid #eee",
                    borderRadius: 10,
                    padding: 10
                  }}
                >
                  <div style={{ display: "flex", gap: 8 }}>
                    <input
                      value={c.name}
                      style={{ ...inputStyle, flex: 1 }}
                      onChange={(e) => updateCategoryLocal(c.id, { name: e.currentTarget.value })}
                      onBlur={() => void updateCategory(c.id, { name: c.name.trim() })}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                    <input
                      value={c.weight == null ? "" : String(c.weight)}
                      style={{ ...inputStyle, width: 90 }}
                      onChange={(e) =>
                        updateCategoryLocal(c.id, {
                          weight: parseNullableNumber(e.currentTarget.value)
                        })
                      }
                      onBlur={() =>
                        void updateCategory(c.id, {
                          weight: c.weight == null ? null : c.weight
                        })
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                    <button
                      onClick={() => void deleteCategory(c.id)}
                      style={{ color: "#b00020" }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              border: "1px solid #ddd",
              borderRadius: 10,
              padding: 16,
              marginBottom: 16
            }}
          >
            <div style={{ fontWeight: 700, marginBottom: 8 }}>Add Assessment</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <input
                value={newTitle}
                onChange={(e) => setNewTitle(e.currentTarget.value)}
                placeholder="Title"
                style={{ ...inputStyle, flex: "2 1 240px" }}
              />
              <input
                value={newDate}
                onChange={(e) => setNewDate(e.currentTarget.value)}
                placeholder="Date (YYYY-MM-DD)"
                style={{ ...inputStyle, flex: "1 1 160px" }}
              />
              <input
                value={newCategoryName2}
                onChange={(e) => setNewCategoryName2(e.currentTarget.value)}
                placeholder="Category"
                style={{ ...inputStyle, flex: "1 1 140px" }}
              />
              <input
                value={newTerm}
                onChange={(e) => setNewTerm(e.currentTarget.value)}
                placeholder="Term"
                style={{ ...inputStyle, width: 90 }}
              />
              <input
                value={newWeight}
                onChange={(e) => setNewWeight(e.currentTarget.value)}
                placeholder="Weight"
                style={{ ...inputStyle, width: 90 }}
              />
              <input
                value={newOutOf}
                onChange={(e) => setNewOutOf(e.currentTarget.value)}
                placeholder="Out of"
                style={{ ...inputStyle, width: 90 }}
              />
              <button disabled={!canAddAssessment} onClick={() => void addAssessment()}>
                Add
              </button>
            </div>
          </div>

          <div
            data-testid="assessments-table-wrap"
            style={{ overflow: "auto", border: "1px solid #eee", borderRadius: 10 }}
          >
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
                  <th style={{ textAlign: "left", padding: 10, width: 60 }}>#</th>
                  <th style={{ textAlign: "left", padding: 10, width: 260 }}>Title</th>
                  <th style={{ textAlign: "left", padding: 10, width: 150 }}>Date</th>
                  <th style={{ textAlign: "left", padding: 10, width: 140 }}>Category</th>
                  <th style={{ textAlign: "left", padding: 10, width: 90 }}>Term</th>
                  <th style={{ textAlign: "left", padding: 10, width: 110 }}>Weight</th>
                  <th style={{ textAlign: "left", padding: 10, width: 110 }}>Out Of</th>
                  <th style={{ textAlign: "left", padding: 10, width: 90 }} title="From .TYP">
                    Type
                  </th>
                  <th style={{ textAlign: "left", padding: 10, width: 220 }}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {assessments.map((a, i) => (
                  <tr
                    key={a.id}
                    data-testid={`assessment-row-${a.id}`}
                    style={{ borderBottom: "1px solid #f0f0f0" }}
                  >
                    <td style={{ padding: 10, color: "#444" }}>{i + 1}</td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-title-${a.id}`}
                        value={a.title}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, { title: e.currentTarget.value })
                        }
                        onBlur={() => void updateAssessment(a.id, { title: a.title.trim() })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-date-${a.id}`}
                        value={a.date ?? ""}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, { date: e.currentTarget.value || null })
                        }
                        onBlur={() =>
                          void updateAssessment(a.id, {
                            date: (a.date ?? "").trim() || null
                          })
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-category-${a.id}`}
                        value={a.categoryName ?? ""}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            categoryName: e.currentTarget.value || null
                          })
                        }
                        onBlur={() =>
                          void updateAssessment(a.id, {
                            categoryName: (a.categoryName ?? "").trim() || null
                          })
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-term-${a.id}`}
                        value={a.term == null ? "" : String(a.term)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            term: parseNullableInt(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { term: a.term })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-weight-${a.id}`}
                        value={a.weight == null ? "" : String(a.weight)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            weight: parseNullableNumber(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { weight: a.weight })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-outof-${a.id}`}
                        value={a.outOf == null ? "" : String(a.outOf)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            outOf: parseNullableNumber(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { outOf: a.outOf })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10, color: "#555" }}>
                      {a.legacyType == null ? "" : String(a.legacyType)}
                    </td>
                    <td style={{ padding: 10 }}>
                      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                        <button
                          data-testid={`assessment-move-up-${a.id}`}
                          disabled={i === 0}
                          onClick={() => void moveAssessment(i, -1)}
                        >
                          Up
                        </button>
                        <button
                          data-testid={`assessment-move-down-${a.id}`}
                          disabled={i === assessments.length - 1}
                          onClick={() => void moveAssessment(i, 1)}
                        >
                          Down
                        </button>
                        <button
                          data-testid={`assessment-delete-${a.id}`}
                          onClick={() => void deleteAssessment(a.id)}
                          style={{ color: "#b00020" }}
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div style={{ marginTop: 12, fontSize: 12, color: "#666" }}>
            Reordering assessments changes column order in the Marks grid.
          </div>
        </div>
      </div>
    </div>
  );
}
