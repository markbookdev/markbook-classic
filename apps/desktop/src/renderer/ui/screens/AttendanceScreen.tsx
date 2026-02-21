import React, { useEffect, useMemo, useState } from "react";
import {
  AttendanceBulkStampDayResultSchema,
  AttendanceMonthOpenResultSchema,
  AttendanceSetStudentDayResultSchema,
  AttendanceSetTypeOfDayResultSchema,
  SetupGetResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type AttendanceStudent = {
  id: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
};

type AttendanceRow = {
  studentId: string;
  dayCodes: string;
};

const MONTHS = [
  { value: "1", label: "January" },
  { value: "2", label: "February" },
  { value: "3", label: "March" },
  { value: "4", label: "April" },
  { value: "5", label: "May" },
  { value: "6", label: "June" },
  { value: "7", label: "July" },
  { value: "8", label: "August" },
  { value: "9", label: "September" },
  { value: "10", label: "October" },
  { value: "11", label: "November" },
  { value: "12", label: "December" }
];

function normalizeCodes(raw: string, days: number) {
  const chars = raw.split("");
  while (chars.length < days) chars.push(" ");
  if (chars.length > days) chars.length = days;
  return chars.join("");
}

function patchCode(raw: string, day: number, value: string, days: number) {
  const chars = normalizeCodes(raw, days).split("");
  chars[day - 1] = value || " ";
  return chars.join("");
}

export function AttendanceScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [month, setMonth] = useState("9");
  const [schoolYearStartMonth, setSchoolYearStartMonth] = useState(9);
  const [daysInMonth, setDaysInMonth] = useState(30);
  const [typeOfDayCodes, setTypeOfDayCodes] = useState("");
  const [students, setStudents] = useState<AttendanceStudent[]>([]);
  const [rows, setRows] = useState<AttendanceRow[]>([]);
  const [selectedStudentIds, setSelectedStudentIds] = useState<Record<string, boolean>>({});
  const [bulkDay, setBulkDay] = useState(1);
  const [bulkCode, setBulkCode] = useState("P");
  const [loading, setLoading] = useState(false);

  const rowByStudentId = useMemo(() => {
    const out: Record<string, AttendanceRow> = {};
    for (const r of rows) out[r.studentId] = r;
    return out;
  }, [rows]);

  async function loadMonth() {
    setLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed(
        "attendance.monthOpen",
        { classId: props.selectedClassId, month },
        AttendanceMonthOpenResultSchema
      );
      setSchoolYearStartMonth(res.schoolYearStartMonth);
      setDaysInMonth(res.daysInMonth);
      setTypeOfDayCodes(normalizeCodes(res.typeOfDayCodes ?? "", res.daysInMonth));
      setStudents(res.students as AttendanceStudent[]);
      setRows(
        res.rows.map((r) => ({
          studentId: r.studentId,
          dayCodes: normalizeCodes(r.dayCodes ?? "", res.daysInMonth)
        }))
      );
      setBulkDay((d) => Math.max(1, Math.min(res.daysInMonth, d)));
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setStudents([]);
      setRows([]);
      setTypeOfDayCodes("");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadMonth();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, month]);

  useEffect(() => {
    let cancelled = false;
    async function loadDefaults() {
      try {
        const setup = await requestParsed("setup.get", {}, SetupGetResultSchema);
        if (cancelled) return;
        setBulkCode((setup.attendance.presentCode || "P").slice(0, 1).toUpperCase());
        setMonth(String(setup.attendance.schoolYearStartMonth || 9));
      } catch {
        if (cancelled) return;
      }
    }
    void loadDefaults();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId]);

  async function saveTypeOfDay(day: number, value: string) {
    const code = value.trim().slice(0, 1).toUpperCase();
    const next = patchCode(typeOfDayCodes, day, code, daysInMonth);
    setTypeOfDayCodes(next);
    try {
      await requestParsed(
        "attendance.setTypeOfDay",
        {
          classId: props.selectedClassId,
          month,
          day,
          code: code || null
        },
        AttendanceSetTypeOfDayResultSchema
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadMonth();
    }
  }

  async function saveStudentDay(studentId: string, day: number, value: string) {
    const code = value.trim().slice(0, 1).toUpperCase();
    setRows((prev) =>
      prev.map((r) =>
        r.studentId === studentId ? { ...r, dayCodes: patchCode(r.dayCodes, day, code, daysInMonth) } : r
      )
    );
    try {
      await requestParsed(
        "attendance.setStudentDay",
        {
          classId: props.selectedClassId,
          month,
          studentId,
          day,
          code: code || null
        },
        AttendanceSetStudentDayResultSchema
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadMonth();
    }
  }

  async function bulkStamp() {
    const studentIds = Object.entries(selectedStudentIds)
      .filter(([, v]) => v)
      .map(([k]) => k);
    if (studentIds.length === 0) return;
    const code = bulkCode.trim().slice(0, 1).toUpperCase();
    props.onError(null);
    try {
      await requestParsed(
        "attendance.bulkStampDay",
        {
          classId: props.selectedClassId,
          month,
          day: bulkDay,
          studentIds,
          code: code || null
        },
        AttendanceBulkStampDayResultSchema
      );
      setRows((prev) =>
        prev.map((r) =>
          studentIds.includes(r.studentId)
            ? { ...r, dayCodes: patchCode(r.dayCodes, bulkDay, code, daysInMonth) }
            : r
        )
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadMonth();
    }
  }

  return (
    <div data-testid="attendance-screen" style={{ padding: 16, overflow: "auto", height: "100%" }}>
      <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 10 }}>
        <div style={{ fontWeight: 700 }}>Attendance</div>
        <label>
          Month{" "}
          <select
            data-testid="attendance-month-select"
            value={month}
            onChange={(e) => setMonth(e.currentTarget.value)}
          >
            {MONTHS.map((m) => (
              <option key={m.value} value={m.value}>
                {m.label}
              </option>
            ))}
          </select>
        </label>
        <button data-testid="attendance-reload-btn" onClick={() => void loadMonth()} disabled={loading}>
          {loading ? "Loading..." : "Reload"}
        </button>
        <div style={{ color: "#555", fontSize: 12 }}>
          School year starts: {schoolYearStartMonth}
        </div>
      </div>

      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 10 }}>
        <div style={{ fontWeight: 600, fontSize: 13 }}>Bulk stamp selected students</div>
        <label>
          Day{" "}
          <select
            data-testid="attendance-bulk-day-select"
            value={bulkDay}
            onChange={(e) => setBulkDay(Number(e.currentTarget.value))}
          >
            {Array.from({ length: daysInMonth }).map((_, i) => (
              <option key={i + 1} value={i + 1}>
                {i + 1}
              </option>
            ))}
          </select>
        </label>
        <label>
          Code{" "}
          <input
            data-testid="attendance-bulk-code-input"
            value={bulkCode}
            onChange={(e) => setBulkCode(e.currentTarget.value.toUpperCase().slice(0, 1))}
            style={{ width: 36 }}
          />
        </label>
        <button data-testid="attendance-bulk-stamp-btn" onClick={() => void bulkStamp()}>
          Stamp
        </button>
      </div>

      <table
        style={{ borderCollapse: "collapse", fontSize: 12, minWidth: 980 }}
        data-testid="attendance-table"
      >
        <thead>
          <tr>
            <th style={{ border: "1px solid #ddd", padding: "4px 6px" }}>Sel</th>
            <th style={{ border: "1px solid #ddd", padding: "4px 6px", minWidth: 220 }}>
              Student
            </th>
            {Array.from({ length: daysInMonth }).map((_, i) => (
              <th key={i + 1} style={{ border: "1px solid #ddd", padding: "4px 6px", width: 32 }}>
                {i + 1}
              </th>
            ))}
          </tr>
          <tr>
            <th style={{ border: "1px solid #ddd", padding: "4px 6px" }} />
            <th style={{ border: "1px solid #ddd", padding: "4px 6px", textAlign: "left" }}>
              Type of Day
            </th>
            {Array.from({ length: daysInMonth }).map((_, i) => {
              const day = i + 1;
              const value = typeOfDayCodes[day - 1] ?? " ";
              return (
                <th key={day} style={{ border: "1px solid #ddd", padding: 1 }}>
                  <input
                    data-testid={`attendance-type-day-input-${day}`}
                    value={value === " " ? "" : value}
                    onChange={(e) => {
                      const code = e.currentTarget.value.toUpperCase().slice(0, 1);
                      setTypeOfDayCodes(patchCode(typeOfDayCodes, day, code, daysInMonth));
                    }}
                    onBlur={(e) => void saveTypeOfDay(day, e.currentTarget.value)}
                    style={{
                      width: 28,
                      border: "none",
                      textAlign: "center",
                      background: "#f6f6f6"
                    }}
                  />
                </th>
              );
            })}
          </tr>
        </thead>
        <tbody>
          {students.map((s) => {
            const row = rowByStudentId[s.id] ?? { studentId: s.id, dayCodes: " ".repeat(daysInMonth) };
            return (
              <tr key={s.id}>
                <td style={{ border: "1px solid #ddd", padding: "2px 4px", textAlign: "center" }}>
                  <input
                    data-testid={`attendance-row-select-${s.id}`}
                    type="checkbox"
                    checked={Boolean(selectedStudentIds[s.id])}
                    onChange={(e) =>
                      setSelectedStudentIds((prev) => ({ ...prev, [s.id]: e.currentTarget.checked }))
                    }
                  />
                </td>
                <td style={{ border: "1px solid #ddd", padding: "2px 6px" }}>
                  {s.displayName}
                  {!s.active ? <span style={{ color: "#777" }}> (inactive)</span> : null}
                </td>
                {Array.from({ length: daysInMonth }).map((_, i) => {
                  const day = i + 1;
                  const value = row.dayCodes[day - 1] ?? " ";
                  return (
                    <td key={day} style={{ border: "1px solid #ddd", padding: 1 }}>
                      <input
                        data-testid={`attendance-student-cell-${s.id}-${day}`}
                        value={value === " " ? "" : value}
                        onChange={(e) => {
                          const code = e.currentTarget.value.toUpperCase().slice(0, 1);
                          setRows((prev) =>
                            prev.map((r) =>
                              r.studentId === s.id
                                ? { ...r, dayCodes: patchCode(r.dayCodes, day, code, daysInMonth) }
                                : r
                            )
                          );
                        }}
                        onBlur={(e) => void saveStudentDay(s.id, day, e.currentTarget.value)}
                        style={{ width: 28, border: "none", textAlign: "center" }}
                      />
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
