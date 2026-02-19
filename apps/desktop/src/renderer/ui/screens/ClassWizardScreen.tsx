import React, { useEffect, useMemo, useState } from "react";
import {
  ClassesCreateFromWizardResultSchema,
  ClassesWizardDefaultsResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

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

export function ClassWizardScreen(props: {
  onError: (msg: string | null) => void;
  onCancel: () => void;
  onCreated: (classId: string) => Promise<void> | void;
}) {
  const [loadingDefaults, setLoadingDefaults] = useState(true);
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
    let cancelled = false;
    async function loadDefaults() {
      setLoadingDefaults(true);
      props.onError(null);
      try {
        const res = await requestParsed(
          "classes.wizardDefaults",
          {},
          ClassesWizardDefaultsResultSchema
        );
        if (!cancelled) {
          setState(toStateFromDefaults(res.defaults));
        }
      } catch (e: any) {
        if (!cancelled) {
          props.onError(e?.message ?? String(e));
        }
      } finally {
        if (!cancelled) {
          setLoadingDefaults(false);
        }
      }
    }
    void loadDefaults();
    return () => {
      cancelled = true;
    };
  }, []);

  const canCreate = useMemo(
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

  const inputStyle: React.CSSProperties = {
    width: "100%",
    padding: "8px 10px",
    border: "1px solid #ddd",
    borderRadius: 6
  };

  return (
    <div data-testid="class-wizard-screen" style={{ padding: 24, maxWidth: 920 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>
        New Class Wizard
      </div>
      <div style={{ color: "#555", marginBottom: 14, fontSize: 13 }}>
        Classroom-first parity flow based on legacy CLLOAD setup.
      </div>
      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          opacity: loadingDefaults ? 0.7 : 1
        }}
      >
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
              onChange={(e) =>
                setState((cur) => ({ ...cur, classCode: e.currentTarget.value }))
              }
              style={inputStyle}
              placeholder="e.g. MAT1"
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            School Year
            <input
              data-testid="class-wizard-school-year"
              value={state.schoolYear}
              onChange={(e) =>
                setState((cur) => ({ ...cur, schoolYear: e.currentTarget.value }))
              }
              style={inputStyle}
              placeholder="e.g. 2025/2026"
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            School Year Start Month
            <input
              data-testid="class-wizard-start-month"
              value={state.schoolYearStartMonth}
              onChange={(e) =>
                setState((cur) => ({
                  ...cur,
                  schoolYearStartMonth: e.currentTarget.value
                }))
              }
              style={inputStyle}
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            School Name
            <input
              data-testid="class-wizard-school-name"
              value={state.schoolName}
              onChange={(e) =>
                setState((cur) => ({ ...cur, schoolName: e.currentTarget.value }))
              }
              style={inputStyle}
            />
          </label>
          <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
            Teacher Name
            <input
              data-testid="class-wizard-teacher-name"
              value={state.teacherName}
              onChange={(e) =>
                setState((cur) => ({ ...cur, teacherName: e.currentTarget.value }))
              }
              style={inputStyle}
            />
          </label>
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
        </div>

        <div style={{ display: "flex", gap: 8, marginTop: 14 }}>
          <button
            data-testid="class-wizard-create-btn"
            onClick={() => void createFromWizard()}
            disabled={!canCreate}
          >
            {saving ? "Creating..." : "Create Class"}
          </button>
          <button data-testid="class-wizard-cancel-btn" onClick={() => props.onCancel()}>
            Cancel
          </button>
        </div>
      </div>
    </div>
  );
}
