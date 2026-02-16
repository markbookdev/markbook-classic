import React, { useEffect, useMemo, useState } from "react";
import { SeatingGetResultSchema, SeatingSaveResultSchema, StudentsListResultSchema } from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type StudentRow = {
  id: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
};

function shuffle<T>(arr: T[]) {
  const out = arr.slice();
  for (let i = out.length - 1; i > 0; i -= 1) {
    const j = Math.floor(Math.random() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }
  return out;
}

export function SeatingPlanScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [rows, setRows] = useState(6);
  const [seatsPerRow, setSeatsPerRow] = useState(5);
  const [blockedSeatCodes, setBlockedSeatCodes] = useState<Set<number>>(new Set());
  const [assignments, setAssignments] = useState<Array<number | null>>([]);
  const [students, setStudents] = useState<StudentRow[]>([]);
  const [selectedSortOrder, setSelectedSortOrder] = useState<number | null>(null);
  const [saving, setSaving] = useState(false);

  const seatCount = rows * seatsPerRow;
  const seatCodeForIndex = (idx: number) => Math.floor(idx / seatsPerRow) * 10 + (idx % seatsPerRow) + 1;

  const bySortOrder = useMemo(() => {
    const out: Record<number, StudentRow> = {};
    for (const s of students) out[s.sortOrder] = s;
    return out;
  }, [students]);

  const assignedSortOrders = useMemo(() => {
    const out = new Set<number>();
    for (const v of assignments) {
      if (v != null) out.add(v);
    }
    return out;
  }, [assignments]);

  const unassigned = useMemo(() => {
    return students
      .filter((s) => !assignedSortOrders.has(s.sortOrder))
      .sort((a, b) => a.displayName.localeCompare(b.displayName));
  }, [students, assignedSortOrders]);

  async function load() {
    props.onError(null);
    try {
      const [plan, sres] = await Promise.all([
        requestParsed("seating.get", { classId: props.selectedClassId }, SeatingGetResultSchema),
        requestParsed("students.list", { classId: props.selectedClassId }, StudentsListResultSchema)
      ]);
      setRows(plan.rows);
      setSeatsPerRow(plan.seatsPerRow);
      setBlockedSeatCodes(new Set(plan.blockedSeatCodes));
      const next = plan.assignments.slice();
      while (next.length < plan.rows * plan.seatsPerRow) next.push(null);
      if (next.length > plan.rows * plan.seatsPerRow) next.length = plan.rows * plan.seatsPerRow;
      setAssignments(next);
      setStudents(sres.students as StudentRow[]);
      setSelectedSortOrder(null);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setAssignments([]);
      setStudents([]);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  function setSeatAssignmentByIndex(idx: number, sortOrder: number | null) {
    setAssignments((prev) => {
      const next = prev.slice();
      while (next.length < seatCount) next.push(null);
      if (next.length > seatCount) next.length = seatCount;
      for (let i = 0; i < next.length; i += 1) {
        if (i !== idx && next[i] === sortOrder) next[i] = null;
      }
      next[idx] = sortOrder;
      return next;
    });
  }

  function toggleBlocked(idx: number) {
    const seatCode = seatCodeForIndex(idx);
    setBlockedSeatCodes((prev) => {
      const next = new Set(prev);
      if (next.has(seatCode)) next.delete(seatCode);
      else next.add(seatCode);
      return next;
    });
    if (selectedSortOrder != null) return;
    setSeatAssignmentByIndex(idx, null);
  }

  function clickSeat(idx: number) {
    const seatCode = seatCodeForIndex(idx);
    if (blockedSeatCodes.has(seatCode)) return;
    const current = assignments[idx] ?? null;
    if (selectedSortOrder != null) {
      setSeatAssignmentByIndex(idx, selectedSortOrder);
      return;
    }
    if (current != null) setSeatAssignmentByIndex(idx, null);
  }

  function placeBySeatOrder(order: number[]) {
    const source = students.slice().sort((a, b) => a.displayName.localeCompare(b.displayName));
    const next = Array.from({ length: seatCount }, () => null as number | null);
    let cursor = 0;
    for (const seatIdx of order) {
      const seatCode = seatIdx + 1;
      if (blockedSeatCodes.has(seatCode)) continue;
      if (cursor >= source.length) break;
      next[seatIdx] = source[cursor].sortOrder;
      cursor += 1;
    }
    setAssignments(next);
  }

  async function save() {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "seating.save",
        {
          classId: props.selectedClassId,
          rows,
          seatsPerRow,
          blockedSeatCodes: Array.from(blockedSeatCodes).sort((a, b) => a - b),
          assignments
        },
        SeatingSaveResultSchema
      );
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  const rowMajor = Array.from({ length: seatCount }, (_, i) => i);
  const colMajor = Array.from({ length: seatCount }, (_, i) => i).sort((a, b) => {
    const aCol = a % seatsPerRow;
    const bCol = b % seatsPerRow;
    if (aCol !== bCol) return aCol - bCol;
    return Math.floor(a / seatsPerRow) - Math.floor(b / seatsPerRow);
  });

  return (
    <div data-testid="seating-screen" style={{ padding: 16, display: "flex", gap: 16, height: "100%" }}>
      <div style={{ flex: 1, minWidth: 0, overflow: "auto" }}>
        <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
          <div style={{ fontWeight: 700 }}>Seating Plan</div>
          <label>
            Rows{" "}
            <input
              data-testid="seating-rows-input"
              type="number"
              min={1}
              value={rows}
              onChange={(e) => {
                const v = Math.max(1, Number(e.currentTarget.value) || 1);
                setRows(v);
              }}
              style={{ width: 64 }}
            />
          </label>
          <label>
            Seats/Row{" "}
            <input
              data-testid="seating-seats-input"
              type="number"
              min={1}
              value={seatsPerRow}
              onChange={(e) => {
                const v = Math.max(1, Number(e.currentTarget.value) || 1);
                setSeatsPerRow(v);
              }}
              style={{ width: 64 }}
            />
          </label>
          <button data-testid="seating-reload-btn" onClick={() => void load()}>
            Reload
          </button>
          <button data-testid="seating-save-btn" onClick={() => void save()} disabled={saving}>
            {saving ? "Saving..." : "Save"}
          </button>
        </div>

        <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
          <button data-testid="seating-auto-front-btn" onClick={() => placeBySeatOrder(rowMajor)}>
            Auto-place (front-to-back)
          </button>
          <button data-testid="seating-auto-left-btn" onClick={() => placeBySeatOrder(colMajor)}>
            Auto-place (left-to-right)
          </button>
          <button
            data-testid="seating-auto-random-btn"
            onClick={() => placeBySeatOrder(shuffle(rowMajor))}
          >
            Auto-place (random)
          </button>
        </div>

        <div
          data-testid="seating-grid"
          style={{
            display: "grid",
            gridTemplateColumns: `repeat(${seatsPerRow}, minmax(120px, 1fr))`,
            gap: 8
          }}
        >
          {Array.from({ length: seatCount }).map((_, idx) => {
            const seatCode = seatCodeForIndex(idx);
            const blocked = blockedSeatCodes.has(seatCode);
            const sortOrder = assignments[idx] ?? null;
            const student = sortOrder == null ? null : bySortOrder[sortOrder] ?? null;
            return (
              <div
                key={seatCode}
                data-testid={`seating-seat-${seatCode}`}
                style={{
                  border: blocked ? "2px dashed #666" : "1px solid #ccc",
                  borderRadius: 8,
                  background: blocked ? "#f1f1f1" : selectedSortOrder != null ? "#f7fbff" : "#fff",
                  minHeight: 74,
                  padding: 6,
                  display: "flex",
                  flexDirection: "column",
                  gap: 6
                }}
              >
                <div style={{ display: "flex", justifyContent: "space-between", fontSize: 11 }}>
                  <span>Seat {seatCode}</span>
                  <button
                    data-testid={`seating-block-toggle-${seatCode}`}
                    onClick={() => toggleBlocked(idx)}
                    style={{ fontSize: 10 }}
                  >
                    {blocked ? "Unblock" : "Block"}
                  </button>
                </div>
                <button
                  data-testid={`seating-seat-assign-${seatCode}`}
                  disabled={blocked}
                  onClick={() => clickSeat(idx)}
                  style={{
                    textAlign: "left",
                    border: "1px solid #ddd",
                    background: blocked ? "#ececec" : "#fff",
                    borderRadius: 6,
                    minHeight: 36
                  }}
                >
                  {blocked ? "X (blocked)" : student ? student.displayName : "(empty)"}
                </button>
              </div>
            );
          })}
        </div>
      </div>

      <div style={{ width: 280, borderLeft: "1px solid #ddd", paddingLeft: 12, overflow: "auto" }}>
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Students</div>
        <div style={{ color: "#666", fontSize: 12, marginBottom: 8 }}>
          Click a student, then click a seat.
        </div>
        <button
          data-testid="seating-clear-selection-btn"
          onClick={() => setSelectedSortOrder(null)}
          style={{ marginBottom: 8 }}
        >
          Clear Selection
        </button>
        <ul style={{ listStyle: "none", padding: 0, margin: 0 }}>
          {students
            .slice()
            .sort((a, b) => a.displayName.localeCompare(b.displayName))
            .map((s) => {
              const assigned = assignedSortOrders.has(s.sortOrder);
              return (
                <li key={s.id} style={{ marginBottom: 6 }}>
                  <button
                    data-testid={`seating-student-${s.id}`}
                    onClick={() => setSelectedSortOrder(s.sortOrder)}
                    style={{
                      width: "100%",
                      textAlign: "left",
                      border: "1px solid #ddd",
                      borderRadius: 6,
                      background:
                        selectedSortOrder === s.sortOrder ? "#dcecff" : assigned ? "#f3f3f3" : "#fff",
                      fontWeight: selectedSortOrder === s.sortOrder ? 700 : 400
                    }}
                  >
                    {s.displayName}
                    {!s.active ? " (inactive)" : ""}
                    {assigned ? " • assigned" : ""}
                  </button>
                </li>
              );
            })}
        </ul>
      </div>
    </div>
  );
}
