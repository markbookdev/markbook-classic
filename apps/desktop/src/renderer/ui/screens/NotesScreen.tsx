import React, { useEffect, useMemo, useState } from "react";
import {
  NotesGetResultSchema,
  NotesUpdateResultSchema,
  StudentsListResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type StudentRow = {
  id: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
};

export function NotesScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [students, setStudents] = useState<StudentRow[]>([]);
  const [notesByStudentId, setNotesByStudentId] = useState<Record<string, string>>({});
  const [selectedStudentId, setSelectedStudentId] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved">("idle");

  const selectedStudent = useMemo(
    () => students.find((s) => s.id === selectedStudentId) ?? null,
    [students, selectedStudentId]
  );
  const savedNote = selectedStudentId ? notesByStudentId[selectedStudentId] ?? "" : "";

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const [studRes, notesRes] = await Promise.all([
        requestParsed("students.list", { classId: props.selectedClassId }, StudentsListResultSchema),
        requestParsed("notes.get", { classId: props.selectedClassId }, NotesGetResultSchema)
      ]);

      const studs = studRes.students.map((s) => ({
        id: s.id,
        displayName: s.displayName,
        sortOrder: s.sortOrder,
        active: s.active
      }));
      studs.sort((a, b) => a.sortOrder - b.sortOrder);
      setStudents(studs);

      const map: Record<string, string> = {};
      for (const n of notesRes.notes) {
        map[n.studentId] = n.note;
      }
      setNotesByStudentId(map);

      setSelectedStudentId((cur) => {
        if (cur && studs.some((s) => s.id === cur)) return cur;
        return studs[0]?.id ?? null;
      });
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setStudents([]);
      setNotesByStudentId({});
      setSelectedStudentId(null);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  // When selection changes (or notes load), reset draft to the saved value.
  useEffect(() => {
    setDraft(savedNote);
    setDirty(false);
    setSaveState("idle");
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedStudentId, savedNote]);

  async function saveNote() {
    if (!selectedStudentId) return;
    setSaveState("saving");
    props.onError(null);
    try {
      await requestParsed(
        "notes.update",
        { classId: props.selectedClassId, studentId: selectedStudentId, note: draft },
        NotesUpdateResultSchema
      );
      setNotesByStudentId((prev) => {
        const next = { ...prev };
        const trimmed = draft.trim();
        if (!trimmed) delete next[selectedStudentId];
        else next[selectedStudentId] = draft;
        return next;
      });
      setDirty(false);
      setSaveState("saved");
      setTimeout(() => setSaveState("idle"), 900);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setSaveState("idle");
      // Reload to avoid desync.
      await load();
    }
  }

  const statusText =
    saveState === "saving" ? "Saving..." : saveState === "saved" ? "Saved" : "";

  return (
    <div style={{ height: "100%", display: "flex", minHeight: 0 }}>
      <div style={{ width: 320, borderRight: "1px solid #ddd", padding: 12, overflow: "auto" }}>
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Students</div>
        {loading ? (
          <div style={{ color: "#666" }}>Loading...</div>
        ) : students.length === 0 ? (
          <div style={{ color: "#666" }}>(none)</div>
        ) : (
          <ul style={{ margin: 0, paddingLeft: 18 }}>
            {students.map((s) => (
              <li key={s.id} style={{ marginBottom: 4, opacity: s.active ? 1 : 0.6 }}>
                <button
                  onClick={() => setSelectedStudentId(s.id)}
                  style={{
                    border: "none",
                    background: "transparent",
                    padding: 0,
                    cursor: "pointer",
                    fontWeight: s.id === selectedStudentId ? 700 : 400,
                    color: s.id === selectedStudentId ? "#111" : "#0b57d0"
                  }}
                  title={s.displayName}
                >
                  {s.displayName}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      <div style={{ flex: 1, minWidth: 0, padding: 16, display: "flex", flexDirection: "column", gap: 10 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div style={{ fontWeight: 800, fontSize: 18 }}>
            Notes{selectedStudent ? `: ${selectedStudent.displayName}` : ""}
          </div>
          <button onClick={() => void load()} disabled={loading}>
            Reload
          </button>
          <button onClick={() => void saveNote()} disabled={!dirty || !selectedStudentId}>
            Save
          </button>
          {statusText ? <div style={{ color: "#666", fontSize: 12 }}>{statusText}</div> : null}
          <div style={{ marginLeft: "auto", color: "#666", fontSize: 12 }}>
            {dirty ? "Unsaved changes" : ""}
          </div>
        </div>

        <textarea
          value={draft}
          disabled={!selectedStudentId}
          onChange={(e) => {
            const v = e.currentTarget.value;
            setDraft(v);
            setDirty(v !== savedNote);
          }}
          onBlur={() => {
            if (dirty) void saveNote();
          }}
          placeholder={selectedStudentId ? "Enter notes..." : "Select a student"}
          style={{
            flex: 1,
            width: "100%",
            minHeight: 240,
            resize: "none",
            padding: 12,
            border: "1px solid #ddd",
            borderRadius: 10,
            fontFamily: "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
            fontSize: 13,
            lineHeight: 1.35
          }}
        />
      </div>
    </div>
  );
}
