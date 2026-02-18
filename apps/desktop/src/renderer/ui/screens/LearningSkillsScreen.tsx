import React, { useEffect, useMemo, useState } from "react";
import {
  LearningSkillsOpenResultSchema,
  LearningSkillsUpdateCellResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type StudentRow = {
  id: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
};

type SkillRow = {
  studentId: string;
  values: Record<string, string>;
};

export function LearningSkillsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [term, setTerm] = useState(1);
  const [skillCodes, setSkillCodes] = useState<string[]>([]);
  const [students, setStudents] = useState<StudentRow[]>([]);
  const [rows, setRows] = useState<SkillRow[]>([]);
  const [loading, setLoading] = useState(false);

  const rowByStudent = useMemo(() => {
    const out: Record<string, SkillRow> = {};
    for (const r of rows) out[r.studentId] = r;
    return out;
  }, [rows]);

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed(
        "learningSkills.open",
        { classId: props.selectedClassId, term },
        LearningSkillsOpenResultSchema
      );
      setSkillCodes(res.skillCodes);
      setStudents(res.students as StudentRow[]);
      setRows(res.rows as SkillRow[]);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setSkillCodes([]);
      setStudents([]);
      setRows([]);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, term]);

  async function saveCell(studentId: string, skillCode: string, value: string) {
    const trimmed = value.trim().toUpperCase().slice(0, 2);
    setRows((prev) =>
      prev.map((r) =>
        r.studentId === studentId
          ? { ...r, values: { ...r.values, [skillCode]: trimmed } }
          : r
      )
    );
    try {
      await requestParsed(
        "learningSkills.updateCell",
        {
          classId: props.selectedClassId,
          studentId,
          term,
          skillCode,
          value: trimmed || null
        },
        LearningSkillsUpdateCellResultSchema
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await load();
    }
  }

  return (
    <div data-testid="learning-skills-screen" style={{ padding: 16, overflow: "auto", height: "100%" }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
        <div style={{ fontWeight: 700 }}>Learning Skills</div>
        <label>
          Term{" "}
          <select
            data-testid="learning-skills-term-select"
            value={term}
            onChange={(e) => setTerm(Number(e.currentTarget.value || 1))}
          >
            <option value={1}>Term 1</option>
            <option value={2}>Term 2</option>
            <option value={3}>Term 3</option>
          </select>
        </label>
        <button data-testid="learning-skills-reload-btn" onClick={() => void load()} disabled={loading}>
          {loading ? "Loading..." : "Reload"}
        </button>
      </div>

      <table style={{ borderCollapse: "collapse", minWidth: 680 }}>
        <thead>
          <tr>
            <th style={{ border: "1px solid #ddd", padding: "6px 8px", textAlign: "left", minWidth: 240 }}>
              Student
            </th>
            {skillCodes.map((code) => (
              <th
                key={code}
                style={{ border: "1px solid #ddd", padding: "6px 8px", minWidth: 72, textAlign: "center" }}
              >
                {code}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {students.map((s) => {
            const values = rowByStudent[s.id]?.values ?? {};
            return (
              <tr key={s.id} style={{ opacity: s.active ? 1 : 0.55 }}>
                <td style={{ border: "1px solid #eee", padding: "6px 8px" }}>{s.displayName}</td>
                {skillCodes.map((code) => (
                  <td key={code} style={{ border: "1px solid #eee", padding: 4 }}>
                    <input
                      data-testid={`ls-cell-${s.id}-${code}`}
                      value={values[code] ?? ""}
                      onChange={(e) =>
                        setRows((prev) =>
                          prev.map((r) =>
                            r.studentId === s.id
                              ? {
                                  ...r,
                                  values: {
                                    ...r.values,
                                    [code]: e.currentTarget.value.toUpperCase().slice(0, 2)
                                  }
                                }
                              : r
                          )
                        )
                      }
                      onBlur={(e) => void saveCell(s.id, code, e.currentTarget.value)}
                      style={{ width: "100%", textAlign: "center", padding: "4px 6px" }}
                    />
                  </td>
                ))}
              </tr>
            );
          })}
        </tbody>
      </table>

      <div style={{ marginTop: 10, color: "#666", fontSize: 12 }}>
        Ratings are stored by term and persist through sidecar IPC.
      </div>
    </div>
  );
}
