import React, { useEffect, useMemo, useState } from "react";
import {
  ClassesCreateFromWizardResultSchema,
  ClassesMetaGetResultSchema,
  ClassesMetaUpdateResultSchema,
  ClassesWizardDefaultsResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type WizardMode = "create" | "edit";

type WizardState = {
  name: string;
  classCode: string;
  schoolYear: string;
  schoolName: string;
  teacherName: string;
  calcMethodDefault: string;
  weightMethodDefault: string;
  schoolYearStartMonth: string;
};

function toStateFromDefaults(defaults: any): WizardState {
  return {
    name: defaults?.name ?? "",
    classCode: defaults?.classCode ?? "",
    schoolYear: defaults?.schoolYear ?? "",
    schoolName: defaults?.schoolName ?? "",
    teacherName: defaults?.teacherName ?? "",
    calcMethodDefault: String(defaults?.calcMethodDefault ?? 0),
    weightMethodDefault: String(defaults?.weightMethodDefault ?? 1),
    schoolYearStartMonth: String(defaults?.schoolYearStartMonth ?? 9)
  };
}

function toStateFromMeta(payload: any): WizardState {
  return {
    name: payload?.class?.name ?? "",
    classCode: payload?.meta?.classCode ?? "",
    schoolYear: payload?.meta?.schoolYear ?? "",
    schoolName: payload?.meta?.schoolName ?? "",
    teacherName: payload?.meta?.teacherName ?? "",
    calcMethodDefault: String(payload?.meta?.calcMethodDefault ?? 0),
    weightMethodDefault: String(payload?.meta?.weightMethodDefault ?? 1),
    schoolYearStartMonth: String(payload?.meta?.schoolYearStartMonth ?? 9)
  };
}

export function ClassWizardScreen(props: {
  onError: (msg: string | null) => void;
  onCancel: () => void;
  onCreated: (classId: string) => Promise<void> | void;
  onMetaSaved?: (classId: string) => Promise<void> | void;
  selectedClassId?: string | null;
  mode?: WizardMode;
}) {
  const [mode, setMode] = useState<WizardMode>(
    props.mode ?? (props.selectedClassId ? "edit" : "create")
  );
  const [step, setStep] = useState(0);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [state, setState] = useState<WizardState>({
    name: "",
    classCode: "",
    schoolYear: "",
    schoolName: "",
    teacherName: "",
    calcMethodDefault: "0",
    weightMethodDefault: "1",
    schoolYearStartMonth: "9"
  });

  useEffect(() => {
    setMode(props.mode ?? (props.selectedClassId ? "edit" : "create"));
  }, [props.mode, props.selectedClassId]);

  useEffect(() => {
    let cancelled = false;
    async function load() {
      setLoading(true);
      props.onError(null);
      try {
        if (mode === "edit" && props.selectedClassId) {
          const res = await requestParsed(
            "classes.meta.get",
            { classId: props.selectedClassId },
            ClassesMetaGetResultSchema
          );
          if (!cancelled) {
            setState(toStateFromMeta(res));
          }
        } else {
          const res = await requestParsed(
            "classes.wizardDefaults",
            {},
            ClassesWizardDefaultsResultSchema
          );
          if (!cancelled) {
            setState(toStateFromDefaults(res.defaults));
          }
        }
      } catch (e: any) {
        if (!cancelled) {
          props.onError(e?.message ?? String(e));
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    }
    void load();
    return () => {
      cancelled = true;
    };
  }, [mode, props.selectedClassId]);

  const canCreate = useMemo(() => {
    if (mode !== "create") return false;
    const validStep0 = state.name.trim().length > 0 && state.classCode.trim().length > 0;
    const validStep1 =
      Number.isInteger(Number(state.calcMethodDefault)) &&
      Number.isInteger(Number(state.weightMethodDefault)) &&
      Number.isInteger(Number(state.schoolYearStartMonth));
    return validStep0 && validStep1 && !saving;
  }, [
    mode,
    state.name,
    state.classCode,
    state.calcMethodDefault,
    state.weightMethodDefault,
    state.schoolYearStartMonth,
    saving
  ]);

  const canSaveMeta = useMemo(
    () => state.name.trim().length > 0 && state.classCode.trim().length > 0 && !saving,
    [state.name, state.classCode, saving]
  );

  async function createFromWizard() {
    if (!canCreate) return;
    setSaving(true);
    props.onError(null);
    try {
      const res = await requestParsed(
        "classes.createFromWizard",
        {
          name: state.name.trim(),
          classCode: state.classCode.trim(),
          schoolYear: state.schoolYear.trim() || null,
          schoolName: state.schoolName.trim() || null,
          teacherName: state.teacherName.trim() || null,
          calcMethodDefault: Number(state.calcMethodDefault),
          weightMethodDefault: Number(state.weightMethodDefault),
          schoolYearStartMonth: Number(state.schoolYearStartMonth)
        },
        ClassesCreateFromWizardResultSchema
      );
      await props.onCreated(res.classId);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function saveClassMeta() {
    if (!props.selectedClassId) return;
    if (!canSaveMeta) return;
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "classes.meta.update",
        {
          classId: props.selectedClassId,
          patch: {
            name: state.name.trim(),
            classCode: state.classCode.trim() || null,
            schoolYear: state.schoolYear.trim() || null,
            schoolName: state.schoolName.trim() || null,
            teacherName: state.teacherName.trim() || null,
            calcMethodDefault: Number(state.calcMethodDefault),
            weightMethodDefault: Number(state.weightMethodDefault),
            schoolYearStartMonth: Number(state.schoolYearStartMonth)
          }
        },
        ClassesMetaUpdateResultSchema
      );
      await props.onMetaSaved?.(props.selectedClassId);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  const inputStyle: React.CSSProperties = {
    width: "100%",
    padding: "8px 10px",
    border: "1px solid #ddd",
    borderRadius: 6
  };

  const stepLabels = ["Class Basics", "Defaults", "Review"];

  const detailsForm = (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        Class Name *
        <input
          data-testid="class-wizard-name"
          value={state.name}
          onChange={(e) => setState((cur) => ({ ...cur, name: e.currentTarget.value }))}
          style={inputStyle}
          placeholder="e.g. 8D (2026)"
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        Class Code *
        <input
          data-testid="class-wizard-code"
          value={state.classCode}
          onChange={(e) => setState((cur) => ({ ...cur, classCode: e.currentTarget.value }))}
          style={inputStyle}
          placeholder="e.g. 8D"
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        School Year
        <input
          data-testid="class-wizard-school-year"
          value={state.schoolYear}
          onChange={(e) => setState((cur) => ({ ...cur, schoolYear: e.currentTarget.value }))}
          style={inputStyle}
          placeholder="e.g. 2025/2026"
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        School Name
        <input
          data-testid="class-wizard-school-name"
          value={state.schoolName}
          onChange={(e) => setState((cur) => ({ ...cur, schoolName: e.currentTarget.value }))}
          style={inputStyle}
        />
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        Teacher Name
        <input
          data-testid="class-wizard-teacher-name"
          value={state.teacherName}
          onChange={(e) => setState((cur) => ({ ...cur, teacherName: e.currentTarget.value }))}
          style={inputStyle}
        />
      </label>
    </div>
  );

  const defaultsForm = (
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        Calc Method Default
        <select
          data-testid="class-wizard-calc-method"
          value={state.calcMethodDefault}
          onChange={(e) =>
            setState((cur) => ({ ...cur, calcMethodDefault: e.currentTarget.value }))
          }
          style={inputStyle}
        >
          <option value="0">Average</option>
          <option value="1">Median</option>
          <option value="2">Mode</option>
          <option value="3">Blended Mode</option>
          <option value="4">Blended Median</option>
        </select>
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        Weight Method Default
        <select
          data-testid="class-wizard-weight-method"
          value={state.weightMethodDefault}
          onChange={(e) =>
            setState((cur) => ({ ...cur, weightMethodDefault: e.currentTarget.value }))
          }
          style={inputStyle}
        >
          <option value="0">Entry</option>
          <option value="1">Category</option>
          <option value="2">Equal</option>
        </select>
      </label>
      <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
        School Year Start Month
        <input
          data-testid="class-wizard-start-month"
          value={state.schoolYearStartMonth}
          onChange={(e) =>
            setState((cur) => ({ ...cur, schoolYearStartMonth: e.currentTarget.value }))
          }
          style={inputStyle}
        />
      </label>
    </div>
  );

  const reviewBlock = (
    <div
      style={{
        border: "1px solid #eee",
        borderRadius: 8,
        padding: 12,
        background: "#fafafa",
        fontSize: 13
      }}
    >
      <div><strong>Class name:</strong> {state.name || "—"}</div>
      <div><strong>Class code:</strong> {state.classCode || "—"}</div>
      <div><strong>School year:</strong> {state.schoolYear || "—"}</div>
      <div><strong>School name:</strong> {state.schoolName || "—"}</div>
      <div><strong>Teacher:</strong> {state.teacherName || "—"}</div>
      <div><strong>Calc method:</strong> {state.calcMethodDefault}</div>
      <div><strong>Weight method:</strong> {state.weightMethodDefault}</div>
      <div><strong>Start month:</strong> {state.schoolYearStartMonth}</div>
    </div>
  );

  return (
    <div data-testid="class-wizard-screen" style={{ padding: 24, maxWidth: 920 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>
        {mode === "create" ? "New Class Wizard" : "Class Profile"}
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button
          data-testid="class-wizard-mode-create"
          onClick={() => {
            setMode("create");
            setStep(0);
          }}
          style={{ fontWeight: mode === "create" ? 700 : 400 }}
        >
          New Class Wizard
        </button>
        <button
          data-testid="class-wizard-mode-edit"
          disabled={!props.selectedClassId}
          onClick={() => {
            if (!props.selectedClassId) return;
            setMode("edit");
          }}
          style={{ fontWeight: mode === "edit" ? 700 : 400 }}
        >
          Edit Class Profile
        </button>
      </div>

      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          opacity: loading ? 0.7 : 1
        }}
      >
        {mode === "create" ? (
          <>
            <div style={{ color: "#555", marginBottom: 14, fontSize: 13 }}>
              Classroom-first parity flow based on legacy CLLOAD setup.
            </div>
            <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
              {stepLabels.map((label, idx) => (
                <div
                  key={label}
                  style={{
                    padding: "4px 8px",
                    borderRadius: 6,
                    border: "1px solid #ddd",
                    background: step === idx ? "#e9f0ff" : "#fff",
                    fontWeight: step === idx ? 700 : 400,
                    fontSize: 12
                  }}
                >
                  {idx + 1}. {label}
                </div>
              ))}
            </div>

            {step === 0 ? detailsForm : step === 1 ? defaultsForm : reviewBlock}

            <div style={{ display: "flex", gap: 8, marginTop: 14 }}>
              <button
                data-testid="class-wizard-back-btn"
                onClick={() => setStep((s) => Math.max(0, s - 1))}
                disabled={step === 0}
              >
                Back
              </button>
              {step < 2 ? (
                <button
                  data-testid="class-wizard-next-btn"
                  onClick={() => setStep((s) => Math.min(2, s + 1))}
                >
                  Next
                </button>
              ) : (
                <button
                  data-testid="class-wizard-create-btn"
                  onClick={() => void createFromWizard()}
                  disabled={!canCreate}
                >
                  {saving ? "Creating..." : "Create Class"}
                </button>
              )}
              <button data-testid="class-wizard-cancel-btn" onClick={() => props.onCancel()}>
                Cancel
              </button>
            </div>
          </>
        ) : (
          <>
            {!props.selectedClassId ? (
              <div style={{ color: "#8a1f11" }}>Select a class first, then open Class Profile.</div>
            ) : (
              <>
                <div style={{ color: "#555", marginBottom: 14, fontSize: 13 }}>
                  Edit class metadata and default calculation settings for the selected class.
                </div>
                {detailsForm}
                <div style={{ height: 10 }} />
                {defaultsForm}
                <div style={{ display: "flex", gap: 8, marginTop: 14 }}>
                  <button
                    data-testid="class-meta-save-btn"
                    onClick={() => void saveClassMeta()}
                    disabled={!canSaveMeta}
                  >
                    {saving ? "Saving..." : "Save Profile"}
                  </button>
                  <button data-testid="class-wizard-cancel-btn" onClick={() => props.onCancel()}>
                    Close
                  </button>
                </div>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}
