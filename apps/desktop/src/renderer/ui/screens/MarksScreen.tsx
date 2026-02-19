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
  CommentsBanksListResultSchema,
  CommentsBanksOpenResultSchema,
  CommentsSetsListResultSchema,
  CommentsSetsOpenResultSchema,
  CommentsRemarksUpsertOneResultSchema,
  GridBulkUpdateResultSchema,
  GridGetResultSchema,
  GridUpdateCellResultSchema,
  MarkSetOpenResultSchema,
  StudentsMembershipGetResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";
import { expandWindow, tilesForWindow, type GridTile } from "../state/marksGridCache";

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

type CommentSetMeta = {
  id: string;
  setNumber: number;
  title: string;
  fitMode: number;
  fitFontSize: number;
  fitWidth: number;
  fitLines: number;
  fitSubj: string;
  maxChars: number;
  isDefault: boolean;
  bankShort: string | null;
};

type BankRow = {
  id: string;
  shortName: string;
  isDefault: boolean;
  fitProfile: string | null;
  sourcePath: string | null;
  entryCount: number;
};

type BankEntry = {
  id: string;
  sortOrder: number;
  typeCode: string;
  levelCode: string;
  text: string;
};

export function MarksScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onGridEvent?: (msg: string) => void;
}) {
  const GRID_TILE_ROWS = 40;
  const GRID_TILE_COLS = 8;
  const GRID_PREFETCH_ROWS = 20;
  const GRID_PREFETCH_COLS = 6;

  const [students, setStudents] = useState<StudentRow[]>([]);
  const [assessments, setAssessments] = useState<AssessmentRow[]>([]);
  const [cells, setCells] = useState<Array<Array<number | null>>>([]);
  const [calcCategories, setCalcCategories] = useState<Array<{ name: string; weight: number }>>(
    []
  );
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
  const [studentCounts, setStudentCounts] = useState<
    Record<string, { noMark: number; zero: number; scored: number }>
  >({});
  const [settingsApplied, setSettingsApplied] = useState<{
    weightMethodApplied: number;
    calcMethodApplied: number;
    roffApplied: boolean;
    modeActiveLevels: number;
  } | null>(null);
  const [perStudentCategories, setPerStudentCategories] = useState<
    Record<
      string,
      Array<{ name: string; value: number | null; weight: number; hasData: boolean }>
    >
  >({});
  const [calcFilters, setCalcFilters] = useState<{
    term: number | null;
    categoryName: string | null;
    typesMask: number | null;
  }>({ term: null, categoryName: null, typesMask: null });
  const [typesSelected, setTypesSelected] = useState<[boolean, boolean, boolean, boolean, boolean]>(
    [true, true, true, true, true]
  );
  const [gridSelection, setGridSelection] = useState<GridSelection>({
    current: undefined,
    rows: CompactSelection.empty(),
    columns: CompactSelection.empty()
  });
  const [selectedCell, setSelectedCell] = useState<{ row: number; col: number } | null>(null);
  const [scoredInput, setScoredInput] = useState("1");

  const [membershipMaskByStudentId, setMembershipMaskByStudentId] = useState<Record<string, string>>({});
  const [membershipMarkSetSortById, setMembershipMarkSetSortById] = useState<Record<string, number>>({});

  const [commentSets, setCommentSets] = useState<CommentSetMeta[]>([]);
  const [selectedCommentSetNumber, setSelectedCommentSetNumber] = useState<number | null>(null);
  const [selectedCommentSetMeta, setSelectedCommentSetMeta] = useState<CommentSetMeta | null>(null);
  const [remarksByStudentId, setRemarksByStudentId] = useState<Record<string, string>>({});
  const [remarkDraft, setRemarkDraft] = useState("");
  const [remarkDirty, setRemarkDirty] = useState(false);
  const [remarkSaveState, setRemarkSaveState] = useState<"idle" | "saving" | "saved">("idle");
  const [remarkApplyMode, setRemarkApplyMode] = useState<"append" | "replace">("append");
  const [banks, setBanks] = useState<BankRow[]>([]);
  const [selectedBankId, setSelectedBankId] = useState<string | null>(null);
  const [bankEntriesByBankId, setBankEntriesByBankId] = useState<Record<string, BankEntry[]>>({});
  const [selectedBankEntryId, setSelectedBankEntryId] = useState<string | null>(null);

  const editorRef = useRef<DataEditorRef | null>(null);
  const [editingCell, setEditingCell] = useState<{
    col: number;
    row: number;
    text: string;
  } | null>(null);
  const editInputRef = useRef<HTMLInputElement | null>(null);
  const loadedTileKeysRef = useRef<Set<string>>(new Set());
  const inflightTileKeysRef = useRef<Set<string>>(new Set());
  const gridTileCacheHitsRef = useRef(0);
  const gridTileCacheMissesRef = useRef(0);
  const gridTileRequestsRef = useRef(0);
  const gridInflightMaxRef = useRef(0);
  const requestEpochRef = useRef(0);
  const [gridGetRequests, setGridGetRequests] = useState(0);

  const selectedStudent =
    selectedCell && selectedCell.row >= 0 && selectedCell.row < students.length
      ? students[selectedCell.row]
      : null;
  const selectedStudentId = selectedStudent?.id ?? null;
  const selectedBankEntries = selectedBankId ? bankEntriesByBankId[selectedBankId] ?? [] : [];
  const selectedBankEntry =
    selectedBankEntries.find((e) => e.id === selectedBankEntryId) ?? selectedBankEntries[0] ?? null;

  // Avoid stale filter closures in async calc refreshes.
  const calcFiltersRef = useRef(calcFilters);
  useEffect(() => {
    calcFiltersRef.current = calcFilters;
  }, [calcFilters]);

  function resetGridCache() {
    loadedTileKeysRef.current = new Set();
    inflightTileKeysRef.current = new Set();
    gridTileCacheHitsRef.current = 0;
    gridTileCacheMissesRef.current = 0;
    gridTileRequestsRef.current = 0;
    gridInflightMaxRef.current = 0;
    setGridGetRequests(0);
  }

  function applyGridSlice(
    prev: Array<Array<number | null>>,
    tile: GridTile,
    slice: Array<Array<number | null>>
  ): Array<Array<number | null>> {
    if (prev.length === 0) return prev;

    const next = prev.slice();
    const rowLimit = Math.min(tile.rowCount, slice.length);
    for (let r = 0; r < rowLimit; r++) {
      const rowIdx = tile.rowStart + r;
      if (rowIdx < 0 || rowIdx >= next.length) continue;
      const rowPrev = next[rowIdx] ? next[rowIdx].slice() : [];
      const source = slice[r] ?? [];
      const colLimit = Math.min(tile.colCount, source.length);
      for (let c = 0; c < colLimit; c++) {
        const colIdx = tile.colStart + c;
        if (colIdx < 0) continue;
        rowPrev[colIdx] = source[c] ?? null;
      }
      next[rowIdx] = rowPrev;
    }
    return next;
  }

  async function fetchGridTile(tile: GridTile, requestEpoch: number) {
    if (requestEpoch !== requestEpochRef.current) return;
    if (loadedTileKeysRef.current.has(tile.key)) return;
    if (inflightTileKeysRef.current.has(tile.key)) return;

    inflightTileKeysRef.current.add(tile.key);
    gridTileRequestsRef.current += 1;
    if (inflightTileKeysRef.current.size > gridInflightMaxRef.current) {
      gridInflightMaxRef.current = inflightTileKeysRef.current.size;
    }
    props.onGridEvent?.(
      `grid.get r${tile.rowStart}+${tile.rowCount} c${tile.colStart}+${tile.colCount}`
    );
    setGridGetRequests((x) => x + 1);

    try {
      const grid = await requestParsed(
        "grid.get",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          rowStart: tile.rowStart,
          rowCount: tile.rowCount,
          colStart: tile.colStart,
          colCount: tile.colCount
        },
        GridGetResultSchema
      );

      if (requestEpoch !== requestEpochRef.current) return;
      setCells((prev) => applyGridSlice(prev, tile, grid.cells));
      loadedTileKeysRef.current.add(tile.key);
    } catch (e: any) {
      if (requestEpoch !== requestEpochRef.current) return;
      props.onError(e?.message ?? String(e));
    } finally {
      inflightTileKeysRef.current.delete(tile.key);
    }
  }

  function ensureGridWindowLoaded(
    window: {
      rowStart: number;
      rowCount: number;
      colStart: number;
      colCount: number;
    },
    dims?: { rowCount: number; colCount: number }
  ) {
    const totalRows = dims?.rowCount ?? students.length;
    const totalCols = dims?.colCount ?? assessments.length;
    if (totalRows <= 0 || totalCols <= 0) return;

    const expanded = expandWindow(
      window,
      totalRows,
      totalCols,
      GRID_PREFETCH_ROWS,
      GRID_PREFETCH_COLS
    );
    const tiles = tilesForWindow(
      expanded,
      totalRows,
      totalCols,
      GRID_TILE_ROWS,
      GRID_TILE_COLS
    );
    const epoch = requestEpochRef.current;
    for (const tile of tiles) {
      if (loadedTileKeysRef.current.has(tile.key) || inflightTileKeysRef.current.has(tile.key)) {
        gridTileCacheHitsRef.current += 1;
        continue;
      }
      gridTileCacheMissesRef.current += 1;
      void fetchGridTile(tile, epoch);
    }
  }

  async function refreshCalcViews() {
    try {
      const [stats, summary] = await Promise.all([
        requestParsed(
          "calc.assessmentStats",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            filters: calcFiltersRef.current
          },
          CalcAssessmentStatsResultSchema
        ),
        requestParsed(
          "calc.markSetSummary",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId,
            filters: calcFiltersRef.current
          },
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
      setStudentCounts(
        Object.fromEntries(
          summary.perStudent.map((s) => [
            s.studentId,
            { noMark: s.noMarkCount, zero: s.zeroCount, scored: s.scoredCount }
          ])
        )
      );
      setCalcCategories(summary.categories.map((c) => ({ name: c.name, weight: c.weight })));
      setSettingsApplied(
        summary.settingsApplied
          ? {
              weightMethodApplied: summary.settingsApplied.weightMethodApplied,
              calcMethodApplied: summary.settingsApplied.calcMethodApplied,
              roffApplied: summary.settingsApplied.roffApplied,
              modeActiveLevels: summary.settingsApplied.modeActiveLevels
            }
          : null
      );
      setPerStudentCategories(
        Object.fromEntries(
          (summary.perStudentCategories ?? []).map((row) => [
            row.studentId,
            row.categories
          ])
        )
      );
    } catch {
      // Keep existing values if calc endpoints fail.
    }
  }

  async function refreshMembership() {
    try {
      const res = await requestParsed(
        "students.membership.get",
        { classId: props.selectedClassId },
        StudentsMembershipGetResultSchema
      );
      setMembershipMaskByStudentId(
        Object.fromEntries(res.students.map((s) => [s.id, s.mask]))
      );
      setMembershipMarkSetSortById(
        Object.fromEntries(res.markSets.map((ms) => [ms.id, ms.sortOrder]))
      );
    } catch {
      setMembershipMaskByStudentId({});
      setMembershipMarkSetSortById({});
    }
  }

  async function loadCommentSet(setNumber: number, availableBanks: BankRow[] = banks) {
    const open = await requestParsed(
      "comments.sets.open",
      {
        classId: props.selectedClassId,
        markSetId: props.selectedMarkSetId,
        setNumber
      },
      CommentsSetsOpenResultSchema
    );
    const setMeta = open.set as CommentSetMeta;
    setSelectedCommentSetMeta(setMeta);
    setSelectedCommentSetNumber(setMeta.setNumber);
    setRemarksByStudentId(
      Object.fromEntries(
        open.remarksByStudent.map((r) => [r.studentId, r.remark])
      )
    );
    const linked = setMeta.bankShort
      ? availableBanks.find(
          (b) => b.shortName.toUpperCase() === setMeta.bankShort?.toUpperCase()
        )
      : null;
    setSelectedBankId((cur) => {
      if (linked) return linked.id;
      if (cur && availableBanks.some((b) => b.id === cur)) return cur;
      return availableBanks[0]?.id ?? null;
    });
  }

  async function refreshCommentsAndBanks() {
    try {
      const [setsRes, banksRes] = await Promise.all([
        requestParsed(
          "comments.sets.list",
          {
            classId: props.selectedClassId,
            markSetId: props.selectedMarkSetId
          },
          CommentsSetsListResultSchema
        ),
        requestParsed("comments.banks.list", {}, CommentsBanksListResultSchema)
      ]);

      const nextSets = setsRes.sets as CommentSetMeta[];
      const nextBanks = banksRes.banks as BankRow[];
      setCommentSets(nextSets);
      setBanks(nextBanks);

      const nextSetNumber =
        selectedCommentSetNumber != null &&
        nextSets.some((s) => s.setNumber === selectedCommentSetNumber)
          ? selectedCommentSetNumber
          : (nextSets[0]?.setNumber ?? null);

      if (nextSetNumber == null) {
        setSelectedCommentSetNumber(null);
        setSelectedCommentSetMeta(null);
        setRemarksByStudentId({});
      } else {
        await loadCommentSet(nextSetNumber, nextBanks);
      }

    } catch {
      setCommentSets([]);
      setSelectedCommentSetNumber(null);
      setSelectedCommentSetMeta(null);
      setRemarksByStudentId({});
      setBanks([]);
      setSelectedBankId(null);
      setBankEntriesByBankId({});
      setSelectedBankEntryId(null);
    }
  }

  async function ensureBankEntriesLoaded(bankId: string) {
    if (!bankId) return;
    if (bankEntriesByBankId[bankId]) return;
    const res = await requestParsed(
      "comments.banks.open",
      { bankId },
      CommentsBanksOpenResultSchema
    );
    const entries = (res.entries as BankEntry[]).slice().sort((a, b) => a.sortOrder - b.sortOrder);
    setBankEntriesByBankId((prev) => ({ ...prev, [bankId]: entries }));
    setSelectedBankEntryId((cur) => {
      if (cur && entries.some((e) => e.id === cur)) return cur;
      return entries[0]?.id ?? null;
    });
  }

  // When type checkboxes change, derive the bitmask expected by the sidecar.
  useEffect(() => {
    const all = typesSelected.every(Boolean);
    if (all) {
      setCalcFilters((cur) => ({ ...cur, typesMask: null }));
      return;
    }
    let mask = 0;
    for (let i = 0; i < typesSelected.length; i += 1) {
      if (typesSelected[i]) mask |= 1 << i;
    }
    setCalcFilters((cur) => ({ ...cur, typesMask: mask }));
  }, [typesSelected]);

  // Recompute calc views when filters change.
  useEffect(() => {
    void refreshCalcViews();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [calcFilters.term, calcFilters.categoryName, calcFilters.typesMask]);

  // Valid-kid visibility: load membership masks for this class.
  useEffect(() => {
    void refreshMembership();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  // Load comments + banks for the current class/mark set.
  useEffect(() => {
    void refreshCommentsAndBanks();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, props.selectedMarkSetId]);

  useEffect(() => {
    if (selectedCommentSetNumber == null) return;
    if (selectedCommentSetMeta?.setNumber === selectedCommentSetNumber) return;
    void loadCommentSet(selectedCommentSetNumber);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedCommentSetNumber]);

  useEffect(() => {
    if (!selectedBankId) {
      setSelectedBankEntryId(null);
      return;
    }
    const existing = bankEntriesByBankId[selectedBankId];
    if (existing) {
      setSelectedBankEntryId((cur) => {
        if (cur && existing.some((e) => e.id === cur)) return cur;
        return existing[0]?.id ?? null;
      });
      return;
    }
    void ensureBankEntriesLoaded(selectedBankId);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedBankId, bankEntriesByBankId]);

  useEffect(() => {
    if (!selectedStudentId) {
      setRemarkDraft("");
      setRemarkDirty(false);
      setRemarkSaveState("idle");
      return;
    }
    setRemarkDraft(remarksByStudentId[selectedStudentId] ?? "");
    setRemarkDirty(false);
    setRemarkSaveState("idle");
  }, [selectedStudentId, remarksByStudentId, selectedCommentSetNumber]);

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
      const res = await requestParsed(
        "grid.bulkUpdate",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          edits
        },
        GridBulkUpdateResultSchema
      );

      const failedKeys = new Set<string>(
        (res.errors ?? []).map((e) => `${e.row}:${e.col}`)
      );
      const acceptedEdits =
        failedKeys.size === 0
          ? edits
          : edits.filter((e) => !failedKeys.has(`${e.row}:${e.col}`));
      applyEditsLocally(acceptedEdits);

      if ((res.rejected ?? 0) > 0) {
        const first = res.errors?.[0];
        const firstMsg = first
          ? ` first error at row ${first.row + 1}, col ${first.col + 1}: ${first.message}`
          : "";
        props.onError(`Bulk edit rejected ${res.rejected} cell(s).${firstMsg}`);
      }
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

  function applySelectedBankEntry() {
    if (!selectedBankEntry) return;
    const bankText = selectedBankEntry.text?.trim();
    if (!bankText) return;
    setRemarkDraft((cur) => {
      if (remarkApplyMode === "replace") return bankText;
      const t = cur.trim();
      return t.length > 0 ? `${cur}${cur.endsWith(" ") ? "" : " "}${bankText}` : bankText;
    });
    setRemarkDirty(true);
    setRemarkSaveState("idle");
  }

  async function saveSelectedStudentRemark() {
    if (!selectedStudentId) return;
    if (!selectedCommentSetMeta) return;
    props.onError(null);
    setRemarkSaveState("saving");
    try {
      await requestParsed(
        "comments.remarks.upsertOne",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          setNumber: selectedCommentSetMeta.setNumber,
          studentId: selectedStudentId,
          remark: remarkDraft
        },
        CommentsRemarksUpsertOneResultSchema
      );
      setRemarksByStudentId((prev) => ({ ...prev, [selectedStudentId]: remarkDraft }));
      setRemarkDirty(false);
      setRemarkSaveState("saved");
    } catch (e: any) {
      setRemarkSaveState("idle");
      props.onError(e?.message ?? String(e));
    }
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
        // Bounds can be null for virtualized rows until scrolled into view.
        editorRef.current?.scrollTo(col, row);
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
      setSelectedCell({ row, col });
      openEditorAt(col, row);
      return true;
    };
    w.__markbookTest.getMarksGridDebug = () => ({
      gridGetRequests,
      loadedTiles: loadedTileKeysRef.current.size,
      inflightTiles: inflightTileKeysRef.current.size,
      tileCacheHits: gridTileCacheHitsRef.current,
      tileCacheMisses: gridTileCacheMissesRef.current,
      tileRequests: gridTileRequestsRef.current,
      inflightMax: gridInflightMaxRef.current
    });
    return () => {
      if (w.__markbookTest?.openMarksCellEditor) delete w.__markbookTest.openMarksCellEditor;
      if (w.__markbookTest?.getMarksGridDebug) delete w.__markbookTest.getMarksGridDebug;
    };
  }, [assessments.length, cells, gridGetRequests, students.length]);

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
      requestEpochRef.current += 1;
      const runEpoch = requestEpochRef.current;
      resetGridCache();
      try {
        const open = await requestParsed(
          "markset.open",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          MarkSetOpenResultSchema
        );
        if (cancelled) return;
        if (runEpoch !== requestEpochRef.current) return;
        setStudents(open.students);
        setAssessments(open.assessments);
        setCells(
          Array.from({ length: open.rowCount }, () =>
            Array.from({ length: open.colCount }, () => null)
          )
        );
        // Default to showing results for a student immediately (and keep selection in range
        // across mark-set changes). This also stabilizes E2E by ensuring the results panel
        // always has a deterministic selected row once data is loaded.
        setSelectedCell((cur) => {
          if (open.students.length === 0) return null;
          const row = cur?.row != null ? Math.min(Math.max(cur.row, 0), open.students.length - 1) : 0;
          const col = cur?.col != null ? cur.col : 0;
          return { row, col };
        });
        const initialRows = Math.min(open.rowCount, GRID_TILE_ROWS);
        const initialCols = Math.min(open.colCount, GRID_TILE_COLS);
        if (initialRows > 0 && initialCols > 0) {
          ensureGridWindowLoaded(
            {
            rowStart: 0,
            rowCount: initialRows,
            colStart: 0,
            colCount: initialCols
            },
            { rowCount: open.rowCount, colCount: open.colCount }
          );
        }

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
        setStudentCounts({});
        setSettingsApplied(null);
        setPerStudentCategories({});
        setCalcCategories([]);
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

  function openEditorAt(col: number, row: number) {
    if (col <= 0 || col > assessments.length) return;
    if (row < 0 || row >= students.length) return;
    ensureGridWindowLoaded({
      rowStart: row,
      rowCount: 1,
      colStart: col - 1,
      colCount: 1
    });
    const cur = cells[row]?.[col - 1] ?? null;
    const text = cur == null ? "" : String(cur);
    editorRef.current?.scrollTo(col, row);
    setEditingCell({ col, row, text });
  }

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

  async function commitEditingCell(move: "none" | "down" | "right" | "left" = "none") {
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

      let nextCol = col;
      let nextRow = row;
      if (move === "down") {
        nextRow = Math.min(students.length - 1, row + 1);
      } else if (move === "right") {
        nextCol = Math.min(assessments.length, col + 1);
      } else if (move === "left") {
        nextCol = Math.max(1, col - 1);
      }

      setSelectedCell({ row: nextRow, col: nextCol });
      if (move === "none" || (nextRow === row && nextCol === col)) {
        setEditingCell(null);
      } else {
        openEditorAt(nextCol, nextRow);
      }
      props.onGridEvent?.(`committed r${row} c${col} move=${move}`);
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
        Fetches: {gridGetRequests} |{" "}
        Avg Raw:{" "}
        {assessmentStats.length === 0
          ? "—"
          : assessmentStats
              .slice(0, 4)
              .map((a) => `${a.title}: ${a.avgRaw.toFixed(1)}`)
              .join(" | ")}
        {assessmentStats.length > 4 ? " | ..." : ""}
      </div>

      <div
        data-testid="marks-results-panel"
        style={{
          position: "absolute",
          right: 12,
          top: 52,
          zIndex: 5,
          width: 340,
          background: "rgba(255,255,255,0.96)",
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: "10px 10px",
          fontSize: 12,
          color: "#222",
          boxShadow: "0 8px 20px rgba(0,0,0,0.08)"
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Results</div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 8 }}>
          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            Term
            <select
              data-testid="marks-filter-term"
              value={calcFilters.term == null ? "ALL" : String(calcFilters.term)}
              onChange={(e) => {
                const v = e.currentTarget.value;
                setCalcFilters((cur) => ({
                  ...cur,
                  term: v === "ALL" ? null : Number(v)
                }));
              }}
            >
              <option value="ALL">ALL</option>
              <option value="1">1</option>
              <option value="2">2</option>
              <option value="3">3</option>
            </select>
          </label>

          <label style={{ display: "flex", alignItems: "center", gap: 6 }}>
            Category
            <select
              data-testid="marks-filter-category"
              value={calcFilters.categoryName ?? "ALL"}
              onChange={(e) => {
                const v = e.currentTarget.value;
                setCalcFilters((cur) => ({
                  ...cur,
                  categoryName: v === "ALL" ? null : v
                }));
              }}
            >
              <option value="ALL">ALL</option>
              {calcCategories.map((c) => (
                <option key={c.name} value={c.name}>
                  {c.name}
                </option>
              ))}
            </select>
          </label>
        </div>

        <div style={{ display: "flex", gap: 10, flexWrap: "wrap", marginBottom: 10 }}>
          {[
            ["Summ", 0],
            ["Form", 1],
            ["Diag", 2],
            ["Self", 3],
            ["Peer", 4]
          ].map(([label, idx]) => (
            <label key={String(idx)} style={{ display: "flex", gap: 6, alignItems: "center" }}>
              <input
                data-testid={`marks-filter-type-${idx}`}
                type="checkbox"
                checked={typesSelected[idx as number]}
                onChange={(e) => {
                  const checked = e.currentTarget.checked;
                  setTypesSelected((cur) => {
                    const next = [...cur] as any;
                    next[idx as number] = checked;
                    return next;
                  });
                }}
              />
              {label}
            </label>
          ))}
        </div>

        {selectedCell && selectedCell.row >= 0 && selectedCell.row < students.length ? (
          (() => {
            const s = students[selectedCell.row];
            const counts = studentCounts[s.id] ?? { noMark: 0, zero: 0, scored: 0 };
            const final = studentFinalMarks[s.id] ?? null;
            const cats = perStudentCategories[s.id] ?? [];
            const sortOrder = membershipMarkSetSortById[props.selectedMarkSetId];
            const mask = membershipMaskByStudentId[s.id];
            const membershipEnabled =
              typeof sortOrder === "number" && typeof mask === "string" && sortOrder >= 0
                ? mask[sortOrder] !== "0"
                : true;
            const validKid = s.active && membershipEnabled;
            const invalidReason = !s.active ? "inactive" : !membershipEnabled ? "excluded from this mark set" : null;
            return (
              <div>
                <div style={{ fontWeight: 600, marginBottom: 4 }}>{s.displayName}</div>
                <div style={{ marginBottom: 8, color: validKid ? "#2e7d32" : "#8a1f11" }}>
                  Valid for this mark set:{" "}
                  <strong>{validKid ? "yes" : "no"}</strong>
                  {!validKid && invalidReason ? (
                    <span style={{ color: "#555" }}> ({invalidReason})</span>
                  ) : null}
                </div>
                <div style={{ display: "flex", gap: 12, marginBottom: 8 }}>
                  <div>
                    <div style={{ color: "#666", fontSize: 10 }}>Final</div>
                    <div data-testid="marks-results-final" style={{ fontSize: 16, fontWeight: 700 }}>
                      {final == null ? "—" : final.toFixed(1)}
                    </div>
                  </div>
                  <div style={{ flex: 1 }}>
                    <div style={{ color: "#666", fontSize: 10 }}>Counts</div>
                    <div data-testid="marks-results-counts">
                      scored {counts.scored}, zero {counts.zero}, no-mark {counts.noMark}
                    </div>
                  </div>
                </div>

                {settingsApplied ? (
                  <div style={{ marginBottom: 8, color: "#555" }}>
                    Method: calc {settingsApplied.calcMethodApplied}, wt {settingsApplied.weightMethodApplied},{" "}
                    roff {settingsApplied.roffApplied ? "on" : "off"}, levels {settingsApplied.modeActiveLevels}
                  </div>
                ) : null}

                <div style={{ maxHeight: 220, overflow: "auto", borderTop: "1px solid #eee", paddingTop: 8 }}>
                  <div style={{ color: "#666", fontSize: 10, marginBottom: 4 }}>Categories</div>
                  {cats.length === 0 ? (
                    <div style={{ color: "#888" }}>—</div>
                  ) : (
                    <table style={{ width: "100%", borderCollapse: "collapse" }}>
                      <tbody>
                        {cats.map((c) => (
                          <tr key={c.name}>
                            <td style={{ padding: "2px 0", whiteSpace: "nowrap" }}>{c.name}</td>
                            <td style={{ padding: "2px 0", textAlign: "right" }}>
                              {c.value == null ? "" : c.value.toFixed(1)}
                            </td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  )}
                </div>

                <div style={{ marginTop: 10, borderTop: "1px solid #eee", paddingTop: 8 }}>
                  <div style={{ color: "#666", fontSize: 10, marginBottom: 6 }}>Remarks</div>
                  <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
                    <select
                      data-testid="marks-remark-set-select"
                      value={selectedCommentSetNumber == null ? "" : String(selectedCommentSetNumber)}
                      onChange={(e) => {
                        const n = Number(e.currentTarget.value);
                        setSelectedCommentSetNumber(Number.isFinite(n) ? n : null);
                      }}
                      style={{ flex: 1 }}
                    >
                      {commentSets.length === 0 ? (
                        <option value="">No comment sets</option>
                      ) : (
                        commentSets.map((set) => (
                          <option key={set.setNumber} value={set.setNumber}>
                            Set {set.setNumber}: {set.title}
                          </option>
                        ))
                      )}
                    </select>
                    <select
                      data-testid="marks-remark-append-toggle"
                      value={remarkApplyMode}
                      onChange={(e) =>
                        setRemarkApplyMode(e.currentTarget.value === "replace" ? "replace" : "append")
                      }
                    >
                      <option value="append">Append</option>
                      <option value="replace">Replace</option>
                    </select>
                  </div>

                  <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
                    <select
                      data-testid="marks-remark-bank-select"
                      value={selectedBankId ?? ""}
                      onChange={(e) => setSelectedBankId(e.currentTarget.value || null)}
                      style={{ flex: 1 }}
                    >
                      {banks.length === 0 ? (
                        <option value="">No banks</option>
                      ) : (
                        banks.map((b) => (
                          <option key={b.id} value={b.id}>
                            {b.shortName} ({b.entryCount})
                          </option>
                        ))
                      )}
                    </select>
                    <button
                      data-testid="marks-remark-apply-btn"
                      disabled={!selectedBankEntry}
                      onClick={() => applySelectedBankEntry()}
                    >
                      Insert
                    </button>
                  </div>

                  <div
                    style={{
                      border: "1px solid #eee",
                      borderRadius: 6,
                      maxHeight: 88,
                      overflow: "auto",
                      padding: 4,
                      marginBottom: 6
                    }}
                  >
                    {selectedBankEntries.length === 0 ? (
                      <div style={{ color: "#888", fontSize: 11, padding: 4 }}>No bank entries</div>
                    ) : (
                      selectedBankEntries.slice(0, 20).map((entry) => (
                        <button
                          key={entry.id}
                          data-testid={`marks-remark-entry-${entry.id}`}
                          onClick={() => {
                            setSelectedBankEntryId(entry.id);
                            const bankText = entry.text?.trim();
                            if (!bankText) return;
                            setRemarkDraft((cur) => {
                              if (remarkApplyMode === "replace") return bankText;
                              const t = cur.trim();
                              return t.length > 0
                                ? `${cur}${cur.endsWith(" ") ? "" : " "}${bankText}`
                                : bankText;
                            });
                            setRemarkDirty(true);
                            setRemarkSaveState("idle");
                          }}
                          style={{
                            display: "block",
                            width: "100%",
                            textAlign: "left",
                            border: "1px solid #ddd",
                            borderRadius: 4,
                            margin: "0 0 4px 0",
                            padding: "4px 6px",
                            background: selectedBankEntryId === entry.id ? "#eef5ff" : "#fff",
                            fontSize: 11
                          }}
                        >
                          {entry.text}
                        </button>
                      ))
                    )}
                  </div>

                  <textarea
                    data-testid="marks-remark-textarea"
                    value={remarkDraft}
                    onChange={(e) => {
                      setRemarkDraft(e.currentTarget.value);
                      setRemarkDirty(true);
                      setRemarkSaveState("idle");
                    }}
                    rows={4}
                    style={{ width: "100%", resize: "vertical", boxSizing: "border-box" }}
                    placeholder="Remark for selected student"
                  />
                  <div style={{ marginTop: 6, display: "flex", gap: 8, alignItems: "center" }}>
                    <button
                      data-testid="marks-remark-save-btn"
                      disabled={!selectedStudentId || !selectedCommentSetMeta || !remarkDirty || remarkSaveState === "saving"}
                      onClick={() => void saveSelectedStudentRemark()}
                    >
                      {remarkSaveState === "saving" ? "Saving..." : "Save Remark"}
                    </button>
                    <span style={{ color: "#666", fontSize: 11 }}>
                      {remarkSaveState === "saved" ? "Saved" : remarkDirty ? "Unsaved changes" : ""}
                    </span>
                  </div>
                </div>
              </div>
            );
          })()
        ) : (
          <div style={{ color: "#777" }}>Click a student row to see results.</div>
        )}
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
        onVisibleRegionChanged={(range) => {
          const rowStart = Math.max(0, range.y);
          const rowCount = Math.max(
            0,
            Math.min(students.length - rowStart, range.height)
          );
          if (rowCount <= 0) return;

          const firstMarkCol = Math.max(1, range.x);
          const lastVisibleCol = range.x + range.width - 1;
          const lastMarkCol = Math.min(assessments.length, lastVisibleCol);
          if (lastMarkCol < firstMarkCol) return;

          ensureGridWindowLoaded({
            rowStart,
            rowCount,
            colStart: firstMarkCol - 1,
            colCount: lastMarkCol - firstMarkCol + 1
          });
        }}
        cellActivationBehavior="double-click"
        editOnType={false}
        onCellClicked={(cell) => {
          const [col, row] = cell;
          props.onGridEvent?.(`clicked r${row} c${col}`);
          setSelectedCell({ row, col });
          editorRef.current?.focus();
          if (col > 0 && col <= assessments.length) {
            // Mark columns edit on single-click for classroom-speed entry.
            openEditorAt(col, row);
          } else {
            setEditingCell(null);
          }
        }}
        onCellActivated={(cell) => {
          const [col, row] = cell;
          props.onGridEvent?.(`activated r${row} c${col}`);
          setSelectedCell({ row, col });
          if (col === 0 || col === assessments.length + 1) return;
          openEditorAt(col, row);
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
                void commitEditingCell("down");
              } else if (e.key === "Tab") {
                e.preventDefault();
                void commitEditingCell(e.shiftKey ? "left" : "right");
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
