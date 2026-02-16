import React, { useEffect, useMemo, useState } from "react";
import {
  StudentsCreateResultSchema,
  StudentsDeleteResultSchema,
  StudentsListResultSchema,
  StudentsReorderResultSchema,
  StudentsUpdateResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type StudentRow = {
  id: string;
  lastName: string;
  firstName: string;
  displayName: string;
  studentNo: string | null;
  birthDate: string | null;
  active: boolean;
  sortOrder: number;
};

export function StudentsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
  onChanged?: () => void | Promise<void>;
}) {
  const [loading, setLoading] = useState(false);
  const [students, setStudents] = useState<StudentRow[]>([]);

  const [newLastName, setNewLastName] = useState("");
  const [newFirstName, setNewFirstName] = useState("");
  const [newStudentNo, setNewStudentNo] = useState("");
  const [newBirthDate, setNewBirthDate] = useState("");
  const [newActive, setNewActive] = useState(true);

  const canAdd = useMemo(() => {
    return newLastName.trim().length > 0 && newFirstName.trim().length > 0;
  }, [newLastName, newFirstName]);

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed(
        "students.list",
        { classId: props.selectedClassId },
        StudentsListResultSchema
      );
      setStudents(res.students);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setStudents([]);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  function updateLocal(studentId: string, patch: Partial<StudentRow>) {
    setStudents((prev) =>
      prev.map((s) => {
        if (s.id !== studentId) return s;
        const next = { ...s, ...patch };
        if ("lastName" in patch || "firstName" in patch) {
          next.displayName = `${next.lastName}, ${next.firstName}`;
        }
        return next;
      })
    );
  }

  async function updateStudent(
    studentId: string,
    patch: {
      lastName?: string;
      firstName?: string;
      studentNo?: string | null;
      birthDate?: string | null;
      active?: boolean;
    }
  ) {
    props.onError(null);
    try {
      await requestParsed(
        "students.update",
        { classId: props.selectedClassId, studentId, patch },
        StudentsUpdateResultSchema
      );
      updateLocal(studentId, patch as any);
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await load();
    }
  }

  async function addStudent() {
    if (!canAdd) return;
    props.onError(null);
    try {
      await requestParsed(
        "students.create",
        {
          classId: props.selectedClassId,
          lastName: newLastName.trim(),
          firstName: newFirstName.trim(),
          studentNo: newStudentNo.trim() ? newStudentNo.trim() : null,
          birthDate: newBirthDate.trim() ? newBirthDate.trim() : null,
          active: newActive
        },
        StudentsCreateResultSchema
      );
      setNewLastName("");
      setNewFirstName("");
      setNewStudentNo("");
      setNewBirthDate("");
      setNewActive(true);
      await load();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteStudent(studentId: string) {
    const ok = confirm("Delete this student? Their marks will also be removed.");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "students.delete",
        { classId: props.selectedClassId, studentId },
        StudentsDeleteResultSchema
      );
      await load();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function moveStudent(idx: number, dir: -1 | 1) {
    const nextIdx = idx + dir;
    if (nextIdx < 0 || nextIdx >= students.length) return;
    const next = students.slice();
    const [row] = next.splice(idx, 1);
    next.splice(nextIdx, 0, row);
    const orderedStudentIds = next.map((s) => s.id);

    props.onError(null);
    try {
      await requestParsed(
        "students.reorder",
        { classId: props.selectedClassId, orderedStudentIds },
        StudentsReorderResultSchema
      );
      setStudents(next.map((s, i) => ({ ...s, sortOrder: i })));
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await load();
    }
  }

  return (
    <div data-testid="students-screen" style={{ padding: 24, maxWidth: 980 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Students</div>

      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          marginBottom: 16
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Add Student</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            value={newLastName}
            onChange={(e) => setNewLastName(e.currentTarget.value)}
            placeholder="Last name"
            style={{ flex: "1 1 180px", padding: "6px 8px" }}
          />
          <input
            value={newFirstName}
            onChange={(e) => setNewFirstName(e.currentTarget.value)}
            placeholder="First name"
            style={{ flex: "1 1 180px", padding: "6px 8px" }}
          />
          <input
            value={newStudentNo}
            onChange={(e) => setNewStudentNo(e.currentTarget.value)}
            placeholder="Student no (optional)"
            style={{ flex: "1 1 180px", padding: "6px 8px" }}
          />
          <input
            value={newBirthDate}
            onChange={(e) => setNewBirthDate(e.currentTarget.value)}
            placeholder="Birth date (optional)"
            style={{ flex: "1 1 180px", padding: "6px 8px" }}
          />
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            <input
              type="checkbox"
              checked={newActive}
              onChange={(e) => setNewActive(e.currentTarget.checked)}
            />
            Active
          </label>
          <button disabled={!canAdd} onClick={() => void addStudent()}>
            Add
          </button>
        </div>
        <div style={{ marginTop: 8, fontSize: 12, color: "#666" }}>
          Reorder with up/down arrows to match the legacy row order.
        </div>
      </div>

      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
        <div style={{ color: "#555", fontSize: 13 }}>
          {loading ? "Loading..." : `${students.length} students`}
        </div>
        <button onClick={() => void load()} disabled={loading}>
          Reload
        </button>
      </div>

      <div
        data-testid="students-table-wrap"
        style={{ overflow: "auto", border: "1px solid #eee", borderRadius: 10 }}
      >
        <table style={{ width: "100%", borderCollapse: "collapse" }}>
          <thead>
            <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
              <th style={{ textAlign: "left", padding: 10, width: 60 }}>#</th>
              <th style={{ textAlign: "left", padding: 10, width: 70 }}>Active</th>
              <th style={{ textAlign: "left", padding: 10 }}>Last</th>
              <th style={{ textAlign: "left", padding: 10 }}>First</th>
              <th style={{ textAlign: "left", padding: 10, width: 160 }}>Student No</th>
              <th style={{ textAlign: "left", padding: 10, width: 160 }}>Birth Date</th>
              <th style={{ textAlign: "left", padding: 10, width: 200 }}>Actions</th>
            </tr>
          </thead>
          <tbody>
            {students.map((s, idx) => {
              const rowStyle: React.CSSProperties = {
                borderBottom: "1px solid #f0f0f0",
                opacity: s.active ? 1 : 0.55
              };
              const inputStyle: React.CSSProperties = {
                width: "100%",
                padding: "6px 8px",
                border: "1px solid #ddd",
                borderRadius: 6
              };
              return (
                <tr key={s.id} data-testid={`student-row-${s.id}`} style={rowStyle}>
                  <td style={{ padding: 10, color: "#444" }}>{idx + 1}</td>
                  <td style={{ padding: 10 }}>
                    <input
                      data-testid={`student-active-${s.id}`}
                      type="checkbox"
                      checked={s.active}
                      onChange={(e) =>
                        void updateStudent(s.id, { active: e.currentTarget.checked })
                      }
                    />
                  </td>
                  <td style={{ padding: 10 }}>
                    <input
                      data-testid={`student-last-${s.id}`}
                      value={s.lastName}
                      style={inputStyle}
                      onChange={(e) =>
                        updateLocal(s.id, { lastName: e.currentTarget.value })
                      }
                      onBlur={() => void updateStudent(s.id, { lastName: s.lastName.trim() })}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                  </td>
                  <td style={{ padding: 10 }}>
                    <input
                      data-testid={`student-first-${s.id}`}
                      value={s.firstName}
                      style={inputStyle}
                      onChange={(e) =>
                        updateLocal(s.id, { firstName: e.currentTarget.value })
                      }
                      onBlur={() => void updateStudent(s.id, { firstName: s.firstName.trim() })}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                  </td>
                  <td style={{ padding: 10 }}>
                    <input
                      data-testid={`student-no-${s.id}`}
                      value={s.studentNo ?? ""}
                      style={inputStyle}
                      onChange={(e) =>
                        updateLocal(s.id, {
                          studentNo: e.currentTarget.value || null
                        })
                      }
                      onBlur={() =>
                        void updateStudent(s.id, {
                          studentNo: (s.studentNo ?? "").trim() || null
                        })
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                  </td>
                  <td style={{ padding: 10 }}>
                    <input
                      data-testid={`student-birth-${s.id}`}
                      value={s.birthDate ?? ""}
                      style={inputStyle}
                      onChange={(e) =>
                        updateLocal(s.id, {
                          birthDate: e.currentTarget.value || null
                        })
                      }
                      onBlur={() =>
                        void updateStudent(s.id, {
                          birthDate: (s.birthDate ?? "").trim() || null
                        })
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                  </td>
                  <td style={{ padding: 10 }}>
                    <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                      <button
                        data-testid={`student-move-up-${s.id}`}
                        disabled={idx === 0}
                        onClick={() => void moveStudent(idx, -1)}
                      >
                        Up
                      </button>
                      <button
                        data-testid={`student-move-down-${s.id}`}
                        disabled={idx === students.length - 1}
                        onClick={() => void moveStudent(idx, 1)}
                      >
                        Down
                      </button>
                      <button
                        data-testid={`student-delete-${s.id}`}
                        onClick={() => void deleteStudent(s.id)}
                        style={{ color: "#b00020" }}
                      >
                        Delete
                      </button>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>
    </div>
  );
}
