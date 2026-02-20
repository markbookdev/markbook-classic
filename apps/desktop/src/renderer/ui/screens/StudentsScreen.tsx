import React, { useEffect, useMemo, useState } from "react";
import {
  ClassesMetaGetResultSchema,
  StudentsCreateResultSchema,
  StudentsDeleteResultSchema,
  StudentsListResultSchema,
  StudentsMembershipBulkSetResultSchema,
  StudentsMembershipGetResultSchema,
  StudentsMembershipSetResultSchema,
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

type MembershipMarkSet = { id: string; code: string; sortOrder: number };
type MembershipStudent = {
  id: string;
  displayName: string;
  active: boolean;
  sortOrder: number;
  mask: string;
};

export function StudentsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
  onChanged?: () => void | Promise<void>;
}) {
  const [tab, setTab] = useState<"roster" | "membership">("roster");

  const [loading, setLoading] = useState(false);
  const [students, setStudents] = useState<StudentRow[]>([]);

  const [membershipLoading, setMembershipLoading] = useState(false);
  const [membershipMarkSets, setMembershipMarkSets] = useState<MembershipMarkSet[]>([]);
  const [membershipStudents, setMembershipStudents] = useState<MembershipStudent[]>([]);
  const [importMeta, setImportMeta] = useState<{
    legacyFolderPath?: string | null;
    legacyClFile?: string | null;
    legacyYearToken?: string | null;
    lastImportedAt?: string | null;
    lastImportWarningsCount?: number;
  } | null>(null);

  const [newLastName, setNewLastName] = useState("");
  const [newFirstName, setNewFirstName] = useState("");
  const [newStudentNo, setNewStudentNo] = useState("");
  const [newBirthDate, setNewBirthDate] = useState("");
  const [newActive, setNewActive] = useState(true);

  const canAdd = useMemo(() => {
    return newLastName.trim().length > 0 && newFirstName.trim().length > 0;
  }, [newLastName, newFirstName]);

  async function loadRoster() {
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

  async function loadMembership() {
    setMembershipLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed(
        "students.membership.get",
        { classId: props.selectedClassId },
        StudentsMembershipGetResultSchema
      );
      setMembershipMarkSets(res.markSets);
      setMembershipStudents(res.students);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setMembershipMarkSets([]);
      setMembershipStudents([]);
    } finally {
      setMembershipLoading(false);
    }
  }

  async function loadImportDiagnostics() {
    try {
      const res = await requestParsed(
        "classes.meta.get",
        { classId: props.selectedClassId },
        ClassesMetaGetResultSchema
      );
      setImportMeta({
        legacyFolderPath: res.meta.legacyFolderPath ?? null,
        legacyClFile: res.meta.legacyClFile ?? null,
        legacyYearToken: res.meta.legacyYearToken ?? null,
        lastImportedAt: res.meta.lastImportedAt ?? null,
        lastImportWarningsCount: res.meta.lastImportWarningsCount ?? 0
      });
    } catch {
      setImportMeta(null);
    }
  }

  useEffect(() => {
    void loadRoster();
    void loadMembership();
    void loadImportDiagnostics();
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
      // Active changes affect valid_kid immediately; keep membership table fresh.
      await loadMembership();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadRoster();
      await loadMembership();
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
      await loadRoster();
      await loadMembership();
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
      await loadRoster();
      await loadMembership();
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
      await loadMembership();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadRoster();
      await loadMembership();
    }
  }

  async function setMembership(studentId: string, markSetId: string, enabled: boolean) {
    props.onError(null);
    try {
      const res = await requestParsed(
        "students.membership.set",
        { classId: props.selectedClassId, studentId, markSetId, enabled },
        StudentsMembershipSetResultSchema
      );
      setMembershipStudents((prev) =>
        prev.map((s) => (s.id === studentId ? { ...s, mask: res.mask } : s))
      );
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadMembership();
    }
  }

  async function bulkSetMembershipForMarkSet(markSetId: string, enabled: boolean) {
    if (membershipLoading) return;
    const ms = membershipMarkSets.find((m) => m.id === markSetId);
    if (!ms) return;

    setMembershipLoading(true);
    props.onError(null);
    try {
      const idx = ms.sortOrder;
      const updates = membershipStudents
        .map((s) => {
          const ch = s.mask?.[idx] ?? "1";
          const checked = ch === "1";
          return { studentId: s.id, needsUpdate: checked !== enabled };
        })
        .filter((x) => x.needsUpdate)
        .map((x) => ({ studentId: x.studentId, enabled }));

      if (updates.length === 0) {
        return;
      }

      const result = await requestParsed(
        "students.membership.bulkSet",
        { classId: props.selectedClassId, markSetId, updates },
        StudentsMembershipBulkSetResultSchema
      );
      if ((result.failed?.length ?? 0) > 0) {
        const first = result.failed?.[0];
        props.onError(
          `Updated ${result.updated}; failed ${result.failed?.length}: ${first?.message ?? "unknown"}`
        );
      }
      await loadMembership();
      if (!result.failed || result.failed.length === 0) {
        await props.onChanged?.();
      }
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadMembership();
    } finally {
      setMembershipLoading(false);
    }
  }

  return (
    <div data-testid="students-screen" style={{ padding: 24, maxWidth: 1040 }}>
      <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
        <div style={{ fontWeight: 800, fontSize: 22 }}>Students</div>
        <div style={{ marginLeft: "auto", display: "flex", gap: 8 }}>
          <button
            onClick={() => setTab("roster")}
            style={{
              fontWeight: tab === "roster" ? 700 : 400,
              border: tab === "roster" ? "2px solid #333" : "1px solid #ccc"
            }}
          >
            Roster
          </button>
          <button
            data-testid="students-membership-tab"
            onClick={() => setTab("membership")}
            style={{
              fontWeight: tab === "membership" ? 700 : 400,
              border: tab === "membership" ? "2px solid #333" : "1px solid #ccc"
            }}
          >
            Mark Set Membership
          </button>
        </div>
      </div>

      <div
        data-testid="students-import-diagnostics"
        style={{
          border: "1px solid #e5e5e5",
          borderRadius: 10,
          background: "#fafafa",
          padding: 12,
          marginBottom: 14,
          fontSize: 12,
          color: "#333"
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Legacy Import Diagnostics</div>
        {importMeta?.legacyFolderPath ? (
          <div style={{ display: "grid", gap: 3 }}>
            <div>
              <strong>Folder:</strong> {importMeta.legacyFolderPath}
            </div>
            <div>
              <strong>CL file:</strong> {importMeta.legacyClFile ?? "—"}
            </div>
            <div>
              <strong>Year token:</strong> {importMeta.legacyYearToken ?? "—"}
            </div>
            <div>
              <strong>Last imported:</strong> {importMeta.lastImportedAt ?? "—"}
            </div>
            <div>
              <strong>Warnings:</strong> {importMeta.lastImportWarningsCount ?? 0}
            </div>
          </div>
        ) : (
          <div style={{ color: "#666" }}>
            No update-from-legacy metadata recorded for this class yet.
          </div>
        )}
      </div>

      {tab === "roster" ? (
        <>
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
            <button onClick={() => void loadRoster()} disabled={loading}>
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
                          onChange={(e) => updateLocal(s.id, { lastName: e.currentTarget.value })}
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
                          onBlur={() =>
                            void updateStudent(s.id, { firstName: s.firstName.trim() })
                          }
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
        </>
      ) : (
        <>
          <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 10 }}>
            <div style={{ color: "#555", fontSize: 13 }}>
              {membershipLoading
                ? "Loading..."
                : `${membershipStudents.length} students, ${membershipMarkSets.length} mark sets`}
            </div>
            <button onClick={() => void loadMembership()} disabled={membershipLoading}>
              Reload
            </button>
          </div>

          <div
            data-testid="students-membership-table-wrap"
            style={{ overflow: "auto", border: "1px solid #eee", borderRadius: 10 }}
          >
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
                  <th style={{ textAlign: "left", padding: 10, width: 280 }}>Student</th>
                  {membershipMarkSets.map((ms) => (
                    <th
                      key={ms.id}
                      style={{ textAlign: "center", padding: 10, minWidth: 80 }}
                      title={ms.code}
                    >
                      <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 6 }}>
                        <div style={{ fontWeight: 700 }}>{ms.code}</div>
                        <div style={{ display: "flex", gap: 6 }}>
                          <button
                            data-testid={`membership-enable-all-${ms.id}`}
                            disabled={membershipLoading}
                            onClick={() => void bulkSetMembershipForMarkSet(ms.id, true)}
                            style={{ fontSize: 11 }}
                            title="Enable all students for this mark set"
                          >
                            All
                          </button>
                          <button
                            data-testid={`membership-disable-all-${ms.id}`}
                            disabled={membershipLoading}
                            onClick={() => void bulkSetMembershipForMarkSet(ms.id, false)}
                            style={{ fontSize: 11 }}
                            title="Disable all students for this mark set"
                          >
                            None
                          </button>
                        </div>
                      </div>
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {membershipStudents.map((s) => (
                  <tr
                    key={s.id}
                    style={{
                      borderBottom: "1px solid #f0f0f0",
                      opacity: s.active ? 1 : 0.55
                    }}
                  >
                    <td style={{ padding: 10, whiteSpace: "nowrap" }}>
                      {s.displayName} {!s.active ? "(inactive)" : ""}
                    </td>
                    {membershipMarkSets.map((ms) => {
                      const idx = ms.sortOrder;
                      const ch = s.mask?.[idx] ?? "1";
                      const checked = ch === "1";
                      return (
                        <td key={ms.id} style={{ padding: 10, textAlign: "center" }}>
                          <input
                            data-testid={`student-membership-cell-${s.id}-${ms.id}`}
                            type="checkbox"
                            checked={checked}
                            onChange={(e) =>
                              void setMembership(s.id, ms.id, e.currentTarget.checked)
                            }
                          />
                        </td>
                      );
                    })}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div style={{ marginTop: 10, fontSize: 12, color: "#666" }}>
            Membership controls which students are included in calculations and reports for each
            mark set (legacy valid_kid semantics).
          </div>
        </>
      )}
    </div>
  );
}
