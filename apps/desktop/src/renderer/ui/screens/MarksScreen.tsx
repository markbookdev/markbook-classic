import React, { useEffect, useMemo, useRef, useState } from "react";
import {
  CompactSelection,
  DataEditor,
  DataEditorRef,
  GridSelection,
  Item,
  GridCell,
  GridCellKind,
  GridColumn,
  EditableGridCell
} from "@glideapps/glide-data-grid";
import "@glideapps/glide-data-grid/dist/index.css";
import {
  CalcAssessmentStatsResultSchema,
  CalcMarkSetSummaryResultSchema,
  GridBulkUpdateResultSchema,
  GridGetResultSchema,
  GridSetStateResultSchema,
  GridUpdateCellResultSchema,
  MarkSetOpenResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type StudentRow = {
  id: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
};

type AssessmentRow = {
  id: string;
  idx: number;
  date: string | null;
  categoryName: string | null;
  title: string;
  weight: number | null;
  outOf: number | null;
};

type BulkScoreEdit = {
  row: number;
  col: number;
  state: "scored" | "zero" | "no_mark";
  value: number | null;
};

export function MarksScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onGridEvent?: (msg: string) => void;
}) {
  const [students, setStudents] = useState<StudentRow[]>([]);
  const [assessments, setAssessments] = useState<AssessmentRow[]>([]);
  const [cells, setCells] = useState<Array<Array<number | null>>>([]);
  const [assessmentStats, setAssessmentStats] = useState<
    Array<{
      assessmentId: string;
      idx: number;
      title: string;
      avgRaw: number;
      avgPercent: number;
      medianPercent: number;
      scoredCount: number;
      zeroCount: number;
      noMarkCount: number;
    }>
  >([]);
  const [studentFinalMarks, setStudentFinalMarks] = useState<Record<string, number | null>>({});
  const [gridSelection, setGridSelection] = useState<GridSelection>({
    current: undefined,
    rows: CompactSelection.empty(),
    columns: CompactSelection.empty()
  });
  const [selectedCell, setSelectedCell] = useState<{ row: number; col: number } | null>(null);
  const [scoredInput, setScoredInput] = useState("1");

  const editorRef = useRef<DataEditorRef | null>(null);
  const [editingCell, setEditingCell] = useState<{
    col: number;
    row: number;
    text: string;
  } | null>(null);
  const editInputRef = useRef<HTMLInputElement | null>(null);

  async function refreshCalcViews() {
    try {
      const [stats, summary] = await Promise.all([
        requestParsed(
          "calc.assessmentStats",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          CalcAssessmentStatsResultSchema
        ),
        requestParsed(
          "calc.markSetSummary",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          CalcMarkSetSummaryResultSchema
        )
      ]);
      setAssessmentStats(
        stats.assessments.map((a) => ({
          assessmentId: a.assessmentId,
          idx: a.idx,
          title: a.title,
          avgRaw: a.avgRaw,
          avgPercent: a.avgPercent,
          medianPercent: a.medianPercent,
          scoredCount: a.scoredCount,
          zeroCount: a.zeroCount,
          noMarkCount: a.noMarkCount
        }))
      );
      setStudentFinalMarks(
        Object.fromEntries(
          summary.perStudent.map((s) => [s.studentId, s.finalMark ?? null])
        )
      );
    } catch {
      // Keep existing values if calc endpoints fail.
    }
  }

  function makeEditFromDisplayValue(row: number, gridCol: number, value: number | null): BulkScoreEdit {
    if (value == null) {
      return { row, col: gridCol, state: "no_mark", value: null };
    }
    if (value === 0) {
      return { row, col: gridCol, state: "zero", value: 0 };
    }
    return { row, col: gridCol, state: "scored", value };
  }

  function applyEditsLocally(edits: BulkScoreEdit[]) {
    setCells((prev) => {
      const next = prev.map((r) => r.slice());
      for (const e of edits) {
        if (!next[e.row]) continue;
        next[e.row][e.col] =
          e.state === "no_mark" ? null : e.state === "zero" ? 0 : e.value ?? null;
      }
      return next;
    });
  }

  async function applyBulkEdits(edits: BulkScoreEdit[]) {
    if (edits.length === 0) return;
    props.onError(null);
    try {
      await requestParsed(
        "grid.bulkUpdate",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          edits
        },
        GridBulkUpdateResultSchema
      );
      applyEditsLocally(edits);
      void refreshCalcViews();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function selectedEditableCells(): Array<{ row: number; col: number }> {
    const out: Array<{ row: number; col: number }> = [];
    const r = gridSelection.current?.range;
    if (r) {
      for (let rr = 0; rr < r.height; rr += 1) {
        const row = r.y + rr;
        if (row < 0 || row >= students.length) continue;
        for (let cc = 0; cc < r.width; cc += 1) {
          const col = r.x + cc;
          if (col <= 0 || col > assessments.length) continue;
          out.push({ row, col });
        }
      }
      if (out.length > 0) return out;
    }

    if (selectedCell && selectedCell.col > 0 && selectedCell.col <= assessments.length) {
      return [selectedCell];
    }
    return [];
  }

  async function setSelectedCellsState(
    state: "scored" | "zero" | "no_mark",
    scoreValue: number | null
  ) {
    const targets = selectedEditableCells();
    if (targets.length === 0) return;
    const edits: BulkScoreEdit[] = targets.map((t) => ({
      row: t.row,
      col: t.col - 1,
      state,
      value: scoreValue
    }));
    await applyBulkEdits(edits);
  }

  async function fillDown() {
    const r = gridSelection.current?.range;
    if (!r || r.height <= 1) return;
    const edits: BulkScoreEdit[] = [];
    for (let cc = 0; cc < r.width; cc += 1) {
      const col = r.x + cc;
      if (col <= 0 || col > assessments.length) continue;
      const source = cells[r.y]?.[col - 1] ?? null;
      for (let rr = 1; rr < r.height; rr += 1) {
        const row = r.y + rr;
        if (row < 0 || row >= students.length) continue;
        edits.push(makeEditFromDisplayValue(row, col - 1, source));
      }
    }
    await applyBulkEdits(edits);
  }

  async function fillRight() {
    const r = gridSelection.current?.range;
    if (!r || r.width <= 1) return;
    const edits: BulkScoreEdit[] = [];
    for (let rr = 0; rr < r.height; rr += 1) {
      const row = r.y + rr;
      if (row < 0 || row >= students.length) continue;
      const sourceCol = r.x;
      if (sourceCol <= 0 || sourceCol > assessments.length) continue;
      const source = cells[row]?.[sourceCol - 1] ?? null;
      for (let cc = 1; cc < r.width; cc += 1) {
        const col = r.x + cc;
        if (col <= 0 || col > assessments.length) continue;
        edits.push(makeEditFromDisplayValue(row, col - 1, source));
      }
    }
    await applyBulkEdits(edits);
  }

  function parsePastedValue(raw: string): { state: "scored" | "zero" | "no_mark"; value: number | null } | null {
    const t = raw.trim();
    if (t === "") return { state: "no_mark", value: null };
    const n = Number(t);
    if (!Number.isFinite(n) || n < 0) return null;
    if (n === 0) return { state: "no_mark", value: null };
    return { state: "scored", value: n };
  }

  function onGridPaste(target: Item, values: readonly (readonly string[])[]): boolean {
    const [targetCol, targetRow] = target;
    if (targetCol <= 0 || targetCol > assessments.length) return false;
    const edits: BulkScoreEdit[] = [];
    for (let rr = 0; rr < values.length; rr += 1) {
      const row = targetRow + rr;
      if (row < 0 || row >= students.length) continue;
      const rowVals = values[rr] ?? [];
      for (let cc = 0; cc < rowVals.length; cc += 1) {
        const col = targetCol + cc;
        if (col <= 0 || col > assessments.length) continue;
        const parsed = parsePastedValue(String(rowVals[cc] ?? ""));
        if (!parsed) continue;
        edits.push({
          row,
          col: col - 1,
          state: parsed.state,
          value: parsed.value
        });
      }
    }
    void applyBulkEdits(edits);
    return false;
  }

  // E2E harness: provide a stable way to compute cell bounds for canvas-based GDG.
  useEffect(() => {
    const w = window as any;
    if (!w.__markbookTest) w.__markbookTest = {};
    w.__markbookTest.getMarksCellBounds = (col: number, row: number) => {
      try {
        return editorRef.current?.getBounds(col, row) ?? null;
      } catch {
        return null;
      }
    };
    return () => {
      if (w.__markbookTest?.getMarksCellBounds) delete w.__markbookTest.getMarksCellBounds;
    };
  }, []);

  // E2E harness: deterministic fallback to open our custom cell editor.
  useEffect(() => {
    const w = window as any;
    if (!w.__markbookTest) w.__markbookTest = {};
    w.__markbookTest.openMarksCellEditor = (col: number, row: number) => {
      if (!Number.isInteger(col) || !Number.isInteger(row)) return false;
      if (col <= 0 || col > assessments.length) return false;
      if (row < 0 || row >= students.length) return false;
      const cur = cells[row]?.[col - 1] ?? null;
      const text = cur == null ? "" : String(cur);
      try {
        editorRef.current?.scrollTo(col, row);
      } catch {
        // no-op
      }
      setEditingCell({ col, row, text });
      return true;
    };
    return () => {
      if (w.__markbookTest?.openMarksCellEditor) delete w.__markbookTest.openMarksCellEditor;
    };
  }, [assessments.length, cells, students.length]);

  useEffect(() => {
    function onErr(e: ErrorEvent) {
      const msg = e?.error?.message || e?.message || "Unknown error";
      props.onError(`Renderer error: ${msg}`);
    }
    function onRej(e: PromiseRejectionEvent) {
      const msg =
        (e?.reason && (e.reason.message || String(e.reason))) || "Unhandled rejection";
      props.onError(`Renderer error: ${msg}`);
    }
    window.addEventListener("error", onErr);
    window.addEventListener("unhandledrejection", onRej);
    return () => {
      window.removeEventListener("error", onErr);
      window.removeEventListener("unhandledrejection", onRej);
    };
  }, [props]);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      props.onError(null);
      try {
        const open = await requestParsed(
          "markset.open",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          MarkSetOpenResultSchema
        );
        if (cancelled) return;
        setStudents(open.students);
        setAssessments(open.assessments);

        const grid = await requestParsed(
          "grid.get",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            rowStart: 0,
            rowCount: open.rowCount,
            colStart: 0,
            colCount: open.colCount
          },
          GridGetResultSchema
        );
        if (cancelled) return;
        setCells(grid.cells);

        await refreshCalcViews();
        if (cancelled) return;
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
        setStudents([]);
        setAssessments([]);
        setCells([]);
        setAssessmentStats([]);
        setStudentFinalMarks({});
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, props.selectedMarkSetId]);

  const cols: GridColumn[] = useMemo(() => {
    const out: GridColumn[] = [{ id: "student", title: "Student", width: 240 }];
    for (const a of assessments) {
      const suffix = a.outOf != null ? ` (${a.outOf})` : "";
      out.push({
        id: a.id,
        title: `${a.title}${suffix}`,
        width: 160
      });
    }
    out.push({ id: "__current__", title: "Current", width: 110 });
    return out;
  }, [assessments]);

  const rows = students.length;

  const getCellContent = ([col, row]: readonly [number, number]): GridCell => {
    if (row < 0 || row >= students.length) {
      return { kind: GridCellKind.Text, data: "", displayData: "", allowOverlay: false };
    }

    if (col === 0) {
      const s = students[row];
      return {
        kind: GridCellKind.Text,
        data: s.displayName,
        displayData: s.displayName,
        allowOverlay: false
      };
    }

    const currentCol = assessments.length + 1;
    if (col === currentCol) {
      const s = students[row];
      const v = studentFinalMarks[s.id] ?? null;
      const txt = v == null ? "" : v.toFixed(1);
      return {
        kind: GridCellKind.Text,
        data: txt,
        displayData: txt,
        allowOverlay: false
      };
    }

    const v = cells[row]?.[col - 1] ?? null;
    const txt = v == null ? "" : String(v);
    return {
      kind: GridCellKind.Number,
      // Undefined means "blank" for NumberCell (shows empty, edits as empty).
      data: v == null ? undefined : v,
      displayData: txt,
      // We own editing (input overlay) because GDG overlay editing has been unreliable in this app.
      allowOverlay: false,
      allowNegative: false
    };
  };

  const editBounds = editingCell
    ? editorRef.current?.getBounds(editingCell.col, editingCell.row)
    : undefined;

  useEffect(() => {
    if (!editingCell) return;
    // Focus/select on open only (not on every keystroke).
    queueMicrotask(() => {
      editInputRef.current?.focus();
      editInputRef.current?.select();
    });
  }, [editingCell?.col, editingCell?.row]);

  async function handleCellEdited(
    cell: readonly [number, number],
    newValue: EditableGridCell
  ) {
    const [col, row] = cell;
    if (col === 0) return;
    if (col === assessments.length + 1) return;
    if (row < 0 || row >= students.length) return;
    const gridCol = col - 1;
    if (gridCol < 0 || gridCol >= assessments.length) return;

    // Locked semantics:
    // - blank => No Mark (excluded)
    // - positive => scored
    // - 0 => No Mark (legacy parity)
    // - negative => reject
    let raw: number | null;
    if (newValue.kind === GridCellKind.Number) {
      const n = (newValue as any).data as number | undefined;
      if (n == null) raw = null;
      else if (!Number.isFinite(n)) raw = null;
      else raw = n;
    } else if (newValue.kind === GridCellKind.Text) {
      const s = String(newValue.data ?? "").trim();
      if (s === "") raw = null;
      else {
        const n = Number(s);
        if (!Number.isFinite(n)) {
          props.onError(`Invalid number: "${s}"`);
          return;
        }
        raw = n;
      }
    } else return;

    if (raw != null && raw < 0) {
      props.onError("Negative marks are not allowed.");
      return;
    }

    // 0 behaves as No Mark (blank).
    const toWrite: number | null = raw != null && raw > 0 ? raw : null;

    props.onError(null);
    try {
      await requestParsed(
        "grid.updateCell",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          row,
          col: gridCol,
          value: toWrite,
          editKind: toWrite == null ? "clear" : "set"
        },
        GridUpdateCellResultSchema
      );

      setCells((prev) => {
        const next = prev.map((r) => r.slice());
        if (!next[row]) return prev;
        next[row][gridCol] = toWrite;
        return next;
      });
      void refreshCalcViews();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      // Best effort: reload the single cell from SQLite to avoid desync.
      try {
        const grid = await requestParsed(
          "grid.get",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            rowStart: row,
            rowCount: 1,
            colStart: gridCol,
            colCount: 1
          },
          GridGetResultSchema
        );
        setCells((prev) => {
          const next = prev.map((r) => r.slice());
          if (!next[row] || !grid.cells?.[0]) return prev;
          next[row][gridCol] = grid.cells[0][0] ?? null;
          return next;
        });
      } catch {
        // ignore
      }
    }
  }

  async function commitEditingCell() {
    if (!editingCell) return;
    const { col, row } = editingCell;
    if (col === 0) return;
    if (col === assessments.length + 1) return;
    const gridCol = col - 1;

    const trimmed = editingCell.text.trim();
    let raw: number | null = null;
    if (trimmed === "") raw = null;
    else {
      const n = Number(trimmed);
      if (!Number.isFinite(n)) {
        props.onError(`Invalid number: "${trimmed}"`);
        return;
      }
      raw = n;
    }
    if (raw != null && raw < 0) {
      props.onError("Negative marks are not allowed.");
      return;
    }

    const toWrite: number | null = raw != null && raw > 0 ? raw : null;

    props.onError(null);
    try {
      await requestParsed(
        "grid.updateCell",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          row,
          col: gridCol,
          value: toWrite,
          editKind: toWrite == null ? "clear" : "set"
        },
        GridUpdateCellResultSchema
      );

      setCells((prev) => {
        const next = prev.map((r) => r.slice());
        if (!next[row]) return prev;
        next[row][gridCol] = toWrite;
        return next;
      });
      void refreshCalcViews();
      setEditingCell(null);
      props.onGridEvent?.(`committed r${row} c${col}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  return (
    <div
      data-testid="marks-screen"
      style={{ position: "relative", width: "100%", height: "100%" }}
    >
      <div
        data-testid="marks-bulk-toolbar"
        style={{
          position: "absolute",
          left: 12,
          top: 12,
          zIndex: 6,
          background: "rgba(255,255,255,0.95)",
          border: "1px solid #ddd",
          borderRadius: 8,
          padding: "6px 8px",
          display: "flex",
          alignItems: "center",
          gap: 6
        }}
      >
        <button data-testid="marks-set-no-mark-btn" onClick={() => void setSelectedCellsState("no_mark", null)}>
          Set No Mark
        </button>
        <button data-testid="marks-set-zero-btn" onClick={() => void setSelectedCellsState("zero", 0)}>
          Set Zero
        </button>
        <input
          data-testid="marks-set-scored-input"
          value={scoredInput}
          onChange={(e) => setScoredInput(e.currentTarget.value)}
          style={{ width: 72 }}
        />
        <button
          data-testid="marks-set-scored-btn"
          onClick={() => {
            const n = Number(scoredInput.trim());
            if (!Number.isFinite(n) || n <= 0) {
              props.onError("Scored value must be a positive number.");
              return;
            }
            void setSelectedCellsState("scored", n);
          }}
        >
          Set Scored
        </button>
        <button data-testid="marks-fill-down-btn" onClick={() => void fillDown()}>
          Fill Down
        </button>
        <button data-testid="marks-fill-right-btn" onClick={() => void fillRight()}>
          Fill Right
        </button>
      </div>

      <div
        data-testid="marks-summary-strip"
        style={{
          position: "absolute",
          right: 12,
          top: 12,
          zIndex: 5,
          background: "rgba(255,255,255,0.92)",
          border: "1px solid #ddd",
          borderRadius: 8,
          padding: "6px 8px",
          fontSize: 11,
          color: "#333",
          maxWidth: 520,
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis"
        }}
        title={assessmentStats
          .map((a) => `${a.title}: ${a.avgRaw.toFixed(1)}`)
          .join(" | ")}
      >
        Avg Raw (active):{" "}
        {assessmentStats.length === 0
          ? "—"
          : assessmentStats
              .slice(0, 4)
              .map((a) => `${a.title}: ${a.avgRaw.toFixed(1)}`)
              .join(" | ")}
        {assessmentStats.length > 4 ? " | ..." : ""}
      </div>

      <DataEditor
        ref={editorRef}
        columns={cols}
        rows={rows}
        getCellContent={getCellContent}
        getCellsForSelection={true}
        rangeSelect="multi-rect"
        gridSelection={gridSelection}
        onGridSelectionChange={setGridSelection}
        onPaste={onGridPaste}
        cellActivationBehavior="double-click"
        editOnType={false}
        onCellClicked={(cell) => {
          props.onGridEvent?.(`clicked r${cell[1]} c${cell[0]}`);
          setSelectedCell({ row: cell[1], col: cell[0] });
          editorRef.current?.focus();
        }}
        onCellActivated={(cell) => {
          const [col, row] = cell;
          props.onGridEvent?.(`activated r${row} c${col}`);
          setSelectedCell({ row, col });
          if (col === 0 || col === assessments.length + 1) return;
          const cur = cells[row]?.[col - 1] ?? null;
          const text = cur == null ? "" : String(cur);
          editorRef.current?.scrollTo(col, row);
          setEditingCell({ col, row, text });
        }}
        // Safety: if built-in overlay ever triggers, keep it wired.
        onCellEdited={(cell, newValue) => {
          props.onGridEvent?.(`edited r${cell[1]} c${cell[0]} (${newValue.kind})`);
          void handleCellEdited(cell, newValue);
        }}
      />

      {editingCell && editBounds ? (
        <div
          data-testid="mark-grid-editor-overlay"
          style={{
            position: "fixed",
            left: editBounds.x,
            top: editBounds.y,
            width: editBounds.width,
            height: editBounds.height,
            zIndex: 1000,
            background: "white",
            border: "2px solid #4c8dff",
            boxSizing: "border-box",
            display: "flex",
            alignItems: "center"
          }}
          onMouseDown={(e) => {
            // Keep focus on the input.
            e.stopPropagation();
          }}
        >
          <input
            data-testid="mark-grid-editor-input"
            ref={editInputRef}
            value={editingCell.text}
            type="text"
            inputMode="decimal"
            onChange={(e) =>
              setEditingCell((cur) =>
                cur ? { ...cur, text: e.currentTarget.value } : cur
              )
            }
            onKeyDown={(e) => {
              // Prevent GDG from intercepting keypresses while our editor is open.
              e.stopPropagation();
              if (e.key === "Enter") {
                e.preventDefault();
                void commitEditingCell();
              } else if (e.key === "Escape") {
                e.preventDefault();
                setEditingCell(null);
                props.onGridEvent?.("edit canceled");
              }
            }}
            style={{
              width: "100%",
              height: "100%",
              border: "none",
              outline: "none",
              padding: "0 6px",
              fontSize: 14,
              background: "transparent"
            }}
          />
        </div>
      ) : null}
    </div>
  );
}
