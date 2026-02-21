import React, { useEffect, useMemo, useState } from "react";
import {
  ClassesListResultSchema,
  CommentsBanksCreateResultSchema,
  CommentsBanksEntryDeleteResultSchema,
  CommentsBanksEntryUpsertResultSchema,
  CommentsBanksExportBnkResultSchema,
  CommentsBanksImportBnkResultSchema,
  CommentsBanksListResultSchema,
  CommentsBanksOpenResultSchema,
  CommentsBanksUpdateMetaResultSchema,
  CommentsTransferApplyResultSchema,
  CommentsTransferFloodFillResultSchema,
  CommentsTransferPreviewResultSchema,
  CommentsSetsDeleteResultSchema,
  CommentsSetsListResultSchema,
  CommentsSetsOpenResultSchema,
  CommentsSetsUpsertResultSchema,
  SetupGetResultSchema,
  MarkSetsListResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type SetRow = {
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

type SetDraft = {
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

type RemarkRow = {
  studentId: string;
  displayName: string;
  sortOrder: number;
  active: boolean;
  remark: string;
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

type TransferClassRow = {
  id: string;
  name: string;
};

type TransferMarkSetRow = {
  id: string;
  code: string;
  description: string;
  sortOrder: number;
};

type TransferPreviewRow = {
  sourceStudentId?: string;
  targetStudentId?: string;
  sourceDisplayName?: string;
  targetDisplayName?: string;
  sourceRemark: string;
  targetRemark: string;
  status: "same" | "different" | "source_only" | "target_only" | "unmatched";
};

export function MarkSetCommentsPanel(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
}) {
  const [sets, setSets] = useState<SetRow[]>([]);
  const [selectedSetNumber, setSelectedSetNumber] = useState<number | null>(null);
  const [setDraft, setSetDraft] = useState<SetDraft | null>(null);
  const [remarks, setRemarks] = useState<RemarkRow[]>([]);

  const [banks, setBanks] = useState<BankRow[]>([]);
  const [selectedBankId, setSelectedBankId] = useState<string | null>(null);
  const [selectedBankEntryId, setSelectedBankEntryId] = useState<string | null>(null);
  const [bankMeta, setBankMeta] = useState<{
    id: string;
    shortName: string;
    isDefault: boolean;
    fitProfile: string | null;
    sourcePath: string | null;
  } | null>(null);
  const [bankEntries, setBankEntries] = useState<BankEntry[]>([]);
  const [selectedStudentId, setSelectedStudentId] = useState<string | null>(null);
  const [newBankShortName, setNewBankShortName] = useState("");
  const [importPath, setImportPath] = useState("");
  const [exportPath, setExportPath] = useState("");
  const [applyMode, setApplyMode] = useState<"append" | "replace">("append");
  const [transferOpen, setTransferOpen] = useState(false);
  const [floodOpen, setFloodOpen] = useState(false);
  const [transferClasses, setTransferClasses] = useState<TransferClassRow[]>([]);
  const [transferSourceClassId, setTransferSourceClassId] = useState("");
  const [transferSourceMarkSetId, setTransferSourceMarkSetId] = useState("");
  const [transferSourceMarkSets, setTransferSourceMarkSets] = useState<TransferMarkSetRow[]>([]);
  const [transferSourceSetNumber, setTransferSourceSetNumber] = useState<number | null>(null);
  const [transferSourceSets, setTransferSourceSets] = useState<SetRow[]>([]);
  const [transferMatchMode, setTransferMatchMode] = useState<
    "student_no_then_name" | "name_only"
  >("student_no_then_name");
  const [transferPolicy, setTransferPolicy] = useState<
    "replace" | "append" | "fill_blank" | "source_if_longer"
  >("fill_blank");
  const [transferSeparator, setTransferSeparator] = useState(" ");
  const [transferScope, setTransferScope] = useState<"all_matched" | "selected_target_students">(
    "all_matched"
  );
  const [transferPreviewRows, setTransferPreviewRows] = useState<TransferPreviewRow[]>([]);
  const [transferPreviewCounts, setTransferPreviewCounts] = useState<any | null>(null);
  const [transferSelectedTargetIds, setTransferSelectedTargetIds] = useState<string[]>([]);
  const [commentsDefaults, setCommentsDefaults] = useState({
    defaultSetNumber: 1,
    defaultTransferPolicy: "fill_blank" as "replace" | "append" | "fill_blank" | "source_if_longer",
    defaultAppendSeparator: " ",
    enforceMaxCharsByDefault: true,
    defaultMaxChars: 600
  });

  const selectedBankEntry = useMemo(() => {
    if (!bankEntries.length) return null;
    if (!selectedBankEntryId) return bankEntries[0];
    return bankEntries.find((e) => e.id === selectedBankEntryId) ?? bankEntries[0];
  }, [bankEntries, selectedBankEntryId]);

  async function loadSets() {
    const res = await requestParsed(
      "comments.sets.list",
      { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
      CommentsSetsListResultSchema
    );
    const next = res.sets as SetRow[];
    setSets(next);
    setSelectedSetNumber((cur) => {
      if (cur != null && next.some((s) => s.setNumber === cur)) return cur;
      return next[0]?.setNumber ?? null;
    });
  }

  async function loadSet(setNumber: number) {
    const res = await requestParsed(
      "comments.sets.open",
      {
        classId: props.selectedClassId,
        markSetId: props.selectedMarkSetId,
        setNumber
      },
      CommentsSetsOpenResultSchema
    );
    const s = res.set;
    setSetDraft({
      setNumber: s.setNumber,
      title: s.title,
      fitMode: s.fitMode,
      fitFontSize: s.fitFontSize,
      fitWidth: s.fitWidth,
      fitLines: s.fitLines,
      fitSubj: s.fitSubj,
      maxChars: s.maxChars,
      isDefault: s.isDefault,
      bankShort: s.bankShort
    });
    setRemarks(res.remarksByStudent as RemarkRow[]);
    if (!selectedStudentId && res.remarksByStudent.length > 0) {
      setSelectedStudentId(res.remarksByStudent[0].studentId);
    }
  }

  async function loadBanks() {
    const res = await requestParsed("comments.banks.list", {}, CommentsBanksListResultSchema);
    const next = res.banks as BankRow[];
    setBanks(next);
    setSelectedBankId((cur) => {
      if (cur && next.some((b) => b.id === cur)) return cur;
      return next[0]?.id ?? null;
    });
  }

  async function loadBank(bankId: string) {
    const res = await requestParsed("comments.banks.open", { bankId }, CommentsBanksOpenResultSchema);
    setBankMeta(res.bank);
    const entries = res.entries as BankEntry[];
    setBankEntries(entries);
    setSelectedBankEntryId((cur) => {
      if (cur && entries.some((e) => e.id === cur)) return cur;
      return entries[0]?.id ?? null;
    });
  }

  async function refreshAll() {
    props.onError(null);
    try {
      const [, , setupRes] = await Promise.all([
        loadSets(),
        loadBanks(),
        requestParsed("setup.get", {}, SetupGetResultSchema)
      ]);
      setCommentsDefaults({
        defaultSetNumber: setupRes.comments.defaultSetNumber,
        defaultTransferPolicy: setupRes.comments.defaultTransferPolicy,
        defaultAppendSeparator: setupRes.comments.defaultAppendSeparator,
        enforceMaxCharsByDefault: setupRes.comments.enforceMaxCharsByDefault,
        defaultMaxChars: setupRes.comments.defaultMaxChars
      });
      setTransferPolicy(setupRes.comments.defaultTransferPolicy);
      setTransferSeparator(setupRes.comments.defaultAppendSeparator);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setSets([]);
      setRemarks([]);
      setBanks([]);
      setSetDraft(null);
      setBankMeta(null);
      setBankEntries([]);
    }
  }

  useEffect(() => {
    void refreshAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, props.selectedMarkSetId]);

  useEffect(() => {
    if (selectedSetNumber == null) {
      setSetDraft(null);
      setRemarks([]);
      return;
    }
    void loadSet(selectedSetNumber).catch((e) => props.onError(e?.message ?? String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedSetNumber]);

  useEffect(() => {
    if (!selectedBankId) {
      setBankMeta(null);
      setBankEntries([]);
      return;
    }
    void loadBank(selectedBankId).catch((e) => props.onError(e?.message ?? String(e)));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedBankId]);

  async function saveSet() {
    if (!setDraft) return;
    props.onError(null);
    try {
      const payload = {
        classId: props.selectedClassId,
        markSetId: props.selectedMarkSetId,
        setNumber: setDraft.setNumber,
        title: setDraft.title,
        fitMode: setDraft.fitMode,
        fitFontSize: setDraft.fitFontSize,
        fitWidth: setDraft.fitWidth,
        fitLines: setDraft.fitLines,
        fitSubj: setDraft.fitSubj,
        maxChars: setDraft.maxChars,
        isDefault: setDraft.isDefault,
        bankShort: setDraft.bankShort,
        remarksByStudent: remarks.map((r) => ({ studentId: r.studentId, remark: r.remark }))
      };
      const res = await requestParsed("comments.sets.upsert", payload, CommentsSetsUpsertResultSchema);
      setSelectedSetNumber(res.setNumber);
      await loadSets();
      await loadSet(res.setNumber);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function createSet() {
    props.onError(null);
    try {
      const title = `Comment Set ${sets.length + 1}`;
      const requestedSetNumber =
        sets.length === 0 ||
        !sets.some((s) => s.setNumber === commentsDefaults.defaultSetNumber)
          ? commentsDefaults.defaultSetNumber
          : undefined;
      const res = await requestParsed(
        "comments.sets.upsert",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          setNumber: requestedSetNumber,
          title,
          fitMode: 0,
          fitFontSize: 9,
          fitWidth: 83,
          fitLines: 12,
          fitSubj: "",
          maxChars: commentsDefaults.enforceMaxCharsByDefault
            ? commentsDefaults.defaultMaxChars
            : 100,
          isDefault: sets.length === 0,
          remarksByStudent: []
        },
        CommentsSetsUpsertResultSchema
      );
      await loadSets();
      setSelectedSetNumber(res.setNumber);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteSet() {
    if (selectedSetNumber == null) return;
    if (!confirm(`Delete comment set ${selectedSetNumber}?`)) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.sets.delete",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          setNumber: selectedSetNumber
        },
        CommentsSetsDeleteResultSchema
      );
      await loadSets();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function createBank() {
    const shortName = newBankShortName.trim();
    if (!shortName) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "comments.banks.create",
        { shortName },
        CommentsBanksCreateResultSchema
      );
      setNewBankShortName("");
      await loadBanks();
      setSelectedBankId(res.bankId);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function saveBankMeta() {
    if (!bankMeta) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.banks.updateMeta",
        {
          bankId: bankMeta.id,
          patch: {
            shortName: bankMeta.shortName,
            isDefault: bankMeta.isDefault,
            fitProfile: bankMeta.fitProfile,
            sourcePath: bankMeta.sourcePath
          }
        },
        CommentsBanksUpdateMetaResultSchema
      );
      await loadBanks();
      if (selectedBankId) await loadBank(selectedBankId);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function addEntry() {
    if (!bankMeta) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.banks.entryUpsert",
        {
          bankId: bankMeta.id,
          typeCode: "GEN",
          levelCode: "~",
          text: "New comment"
        },
        CommentsBanksEntryUpsertResultSchema
      );
      await loadBank(bankMeta.id);
      await loadBanks();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function updateEntry(entry: BankEntry) {
    if (!bankMeta) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.banks.entryUpsert",
        {
          bankId: bankMeta.id,
          entryId: entry.id,
          sortOrder: entry.sortOrder,
          typeCode: entry.typeCode,
          levelCode: entry.levelCode,
          text: entry.text
        },
        CommentsBanksEntryUpsertResultSchema
      );
      await loadBank(bankMeta.id);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteEntry(entryId: string) {
    if (!bankMeta) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.banks.entryDelete",
        { bankId: bankMeta.id, entryId },
        CommentsBanksEntryDeleteResultSchema
      );
      await loadBank(bankMeta.id);
      await loadBanks();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function importBnk() {
    const path = importPath.trim();
    if (!path) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "comments.banks.importBnk",
        { path },
        CommentsBanksImportBnkResultSchema
      );
      await loadBanks();
      setSelectedBankId(res.bankId);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function exportBnk() {
    if (!bankMeta) return;
    const path = exportPath.trim();
    if (!path) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.banks.exportBnk",
        { bankId: bankMeta.id, path },
        CommentsBanksExportBnkResultSchema
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function selectAdjacentStudent(direction: -1 | 1) {
    if (remarks.length === 0) return;
    if (!selectedStudentId) {
      setSelectedStudentId(remarks[0].studentId);
      return;
    }
    const idx = remarks.findIndex((r) => r.studentId === selectedStudentId);
    if (idx < 0) {
      setSelectedStudentId(remarks[0].studentId);
      return;
    }
    const next = Math.min(remarks.length - 1, Math.max(0, idx + direction));
    setSelectedStudentId(remarks[next].studentId);
  }

  function applySelectedBankEntryToStudent() {
    if (!selectedStudentId || !selectedBankEntry) return;
    const nextText = selectedBankEntry.text.trim();
    setRemarks((prev) =>
      prev.map((r) => {
        if (r.studentId !== selectedStudentId) return r;
        if (applyMode === "replace") {
          return { ...r, remark: nextText };
        }
        const merged = `${r.remark} ${nextText}`.trim();
        return { ...r, remark: merged };
      })
    );
  }

  async function browseImportBnkPath() {
    const chosen = await window.markbook.files.pickOpen({
      title: "Import Comment Bank (.BNK)",
      filters: [{ name: "BNK Files", extensions: ["bnk", "BNK"] }]
    });
    if (chosen) setImportPath(chosen);
  }

  async function browseExportBnkPath() {
    const defaultPath = bankMeta ? `${bankMeta.shortName || "comments"}.BNK` : "comments.BNK";
    const chosen = await window.markbook.files.pickSave({
      title: "Export Comment Bank (.BNK)",
      defaultPath,
      filters: [{ name: "BNK Files", extensions: ["bnk", "BNK"] }]
    });
    if (chosen) setExportPath(chosen);
  }

  async function loadTransferClasses() {
    const res = await requestParsed("classes.list", {}, ClassesListResultSchema);
    const next = (res.classes as TransferClassRow[]).slice();
    setTransferClasses(next);
    setTransferSourceClassId((cur) => {
      if (cur && next.some((c) => c.id === cur)) return cur;
      return next[0]?.id ?? "";
    });
  }

  async function loadTransferMarkSets(classId: string) {
    if (!classId) {
      setTransferSourceMarkSets([]);
      setTransferSourceMarkSetId("");
      return;
    }
    const res = await requestParsed(
      "marksets.list",
      { classId, includeDeleted: false },
      MarkSetsListResultSchema
    );
    const next = (res.markSets as TransferMarkSetRow[]).slice().sort((a, b) => a.sortOrder - b.sortOrder);
    setTransferSourceMarkSets(next);
    setTransferSourceMarkSetId((cur) => {
      if (cur && next.some((m) => m.id === cur)) return cur;
      return next[0]?.id ?? "";
    });
  }

  async function loadTransferSets(classId: string, markSetId: string) {
    if (!classId || !markSetId) {
      setTransferSourceSets([]);
      setTransferSourceSetNumber(null);
      return;
    }
    const res = await requestParsed(
      "comments.sets.list",
      { classId, markSetId },
      CommentsSetsListResultSchema
    );
    const next = (res.sets as SetRow[]).slice().sort((a, b) => a.setNumber - b.setNumber);
    setTransferSourceSets(next);
    setTransferSourceSetNumber((cur) => {
      if (cur != null && next.some((s) => s.setNumber === cur)) return cur;
      return next[0]?.setNumber ?? null;
    });
  }

  async function openTransferMode() {
    props.onError(null);
    try {
      await loadTransferClasses();
      setTransferPolicy(commentsDefaults.defaultTransferPolicy);
      setTransferSeparator(commentsDefaults.defaultAppendSeparator);
      setTransferOpen(true);
      setTransferPreviewRows([]);
      setTransferPreviewCounts(null);
      setTransferSelectedTargetIds([]);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function runTransferPreview() {
    if (!transferSourceClassId || !transferSourceMarkSetId || transferSourceSetNumber == null) return;
    if (selectedSetNumber == null) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "comments.transfer.preview",
        {
          sourceClassId: transferSourceClassId,
          sourceMarkSetId: transferSourceMarkSetId,
          sourceSetNumber: transferSourceSetNumber,
          targetClassId: props.selectedClassId,
          targetMarkSetId: props.selectedMarkSetId,
          targetSetNumber: selectedSetNumber,
          studentMatchMode: transferMatchMode
        },
        CommentsTransferPreviewResultSchema
      );
      setTransferPreviewCounts(res.counts);
      setTransferPreviewRows(res.rows as TransferPreviewRow[]);
      setTransferSelectedTargetIds(
        (res.rows as TransferPreviewRow[])
          .filter((r) => !!r.targetStudentId && r.status !== "target_only")
          .map((r) => r.targetStudentId as string)
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setTransferPreviewCounts(null);
      setTransferPreviewRows([]);
      setTransferSelectedTargetIds([]);
    }
  }

  async function applyTransferPreview() {
    if (!transferSourceClassId || !transferSourceMarkSetId || transferSourceSetNumber == null) return;
    if (selectedSetNumber == null) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.transfer.apply",
        {
          sourceClassId: transferSourceClassId,
          sourceMarkSetId: transferSourceMarkSetId,
          sourceSetNumber: transferSourceSetNumber,
          targetClassId: props.selectedClassId,
          targetMarkSetId: props.selectedMarkSetId,
          targetSetNumber: selectedSetNumber,
          studentMatchMode: transferMatchMode,
          policy: transferPolicy,
          separator: transferSeparator,
          targetScope: transferScope,
          selectedTargetStudentIds:
            transferScope === "selected_target_students" ? transferSelectedTargetIds : undefined
        },
        CommentsTransferApplyResultSchema
      );
      await loadSet(selectedSetNumber);
      await runTransferPreview();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function applyFloodFillFromSelected() {
    if (!selectedStudentId || selectedSetNumber == null) return;
    const targetIds =
      transferSelectedTargetIds.length > 0
        ? transferSelectedTargetIds.filter((id) => id !== selectedStudentId)
        : remarks.map((r) => r.studentId).filter((id) => id !== selectedStudentId);
    if (targetIds.length === 0) return;
    props.onError(null);
    try {
      await requestParsed(
        "comments.transfer.floodFill",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          setNumber: selectedSetNumber,
          sourceStudentId: selectedStudentId,
          targetStudentIds: targetIds,
          policy: transferPolicy,
          separator: transferSeparator
        },
        CommentsTransferFloodFillResultSchema
      );
      await loadSet(selectedSetNumber);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  useEffect(() => {
    if (!transferOpen) return;
    if (!transferSourceClassId) return;
    void loadTransferMarkSets(transferSourceClassId).catch((e) =>
      props.onError(e?.message ?? String(e))
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [transferOpen, transferSourceClassId]);

  useEffect(() => {
    if (!transferOpen) return;
    if (!transferSourceClassId || !transferSourceMarkSetId) return;
    void loadTransferSets(transferSourceClassId, transferSourceMarkSetId).catch((e) =>
      props.onError(e?.message ?? String(e))
    );
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [transferOpen, transferSourceClassId, transferSourceMarkSetId]);

  return (
    <div data-testid="comments-panel" style={{ display: "flex", gap: 16, minHeight: 0 }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <div style={{ display: "flex", gap: 8, marginBottom: 8 }}>
          <button data-testid="comments-reload-btn" onClick={() => void refreshAll()}>
            Reload
          </button>
          <button data-testid="comments-set-create-btn" onClick={() => void createSet()}>
            New Set
          </button>
          <button data-testid="comments-set-delete-btn" onClick={() => void deleteSet()}>
            Delete Set
          </button>
          <button data-testid="comments-set-save-btn" onClick={() => void saveSet()}>
            Save Set
          </button>
          <button data-testid="comments-transfer-open-btn" onClick={() => void openTransferMode()}>
            Transfer Mode
          </button>
          <button
            data-testid="comments-floodfill-open-btn"
            onClick={() => setFloodOpen((v) => !v)}
          >
            {floodOpen ? "Hide Flood Fill" : "Flood Fill"}
          </button>
        </div>
        <div style={{ display: "flex", gap: 10, alignItems: "center", marginBottom: 10 }}>
          <label>
            Set{" "}
            <select
              data-testid="comments-set-select"
              value={selectedSetNumber ?? ""}
              onChange={(e) => setSelectedSetNumber(Number(e.currentTarget.value))}
            >
              {sets.map((s) => (
                <option key={s.setNumber} value={s.setNumber}>
                  {s.setNumber}: {s.title}
                </option>
              ))}
            </select>
          </label>
          {setDraft ? (
            <input
              data-testid="comments-set-title-input"
              value={setDraft.title}
              onChange={(e) => setSetDraft({ ...setDraft, title: e.currentTarget.value })}
              style={{ flex: 1, minWidth: 260 }}
            />
          ) : null}
        </div>
        {setDraft ? (
          <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 10 }}>
            <label>
              Fit Mode{" "}
              <input
                data-testid="comments-fit-mode-input"
                type="number"
                value={setDraft.fitMode}
                onChange={(e) =>
                  setSetDraft({ ...setDraft, fitMode: Number(e.currentTarget.value) || 0 })
                }
                style={{ width: 70 }}
              />
            </label>
            <label>
              Font{" "}
              <input
                type="number"
                value={setDraft.fitFontSize}
                onChange={(e) =>
                  setSetDraft({ ...setDraft, fitFontSize: Number(e.currentTarget.value) || 9 })
                }
                style={{ width: 70 }}
              />
            </label>
            <label>
              Width{" "}
              <input
                type="number"
                value={setDraft.fitWidth}
                onChange={(e) =>
                  setSetDraft({ ...setDraft, fitWidth: Number(e.currentTarget.value) || 83 })
                }
                style={{ width: 70 }}
              />
            </label>
            <label>
              Lines{" "}
              <input
                type="number"
                value={setDraft.fitLines}
                onChange={(e) =>
                  setSetDraft({ ...setDraft, fitLines: Number(e.currentTarget.value) || 12 })
                }
                style={{ width: 70 }}
              />
            </label>
            <label>
              Max Chars{" "}
              <input
                type="number"
                value={setDraft.maxChars}
                onChange={(e) =>
                  setSetDraft({ ...setDraft, maxChars: Number(e.currentTarget.value) || 100 })
                }
                style={{ width: 90 }}
              />
            </label>
            <label>
              Bank{" "}
              <input
                value={setDraft.bankShort ?? ""}
                onChange={(e) => setSetDraft({ ...setDraft, bankShort: e.currentTarget.value })}
              />
            </label>
            <label>
              <input
                type="checkbox"
                checked={setDraft.isDefault}
                onChange={(e) => setSetDraft({ ...setDraft, isDefault: e.currentTarget.checked })}
              />{" "}
              Default
            </label>
          </div>
        ) : null}
        <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
          <button
            data-testid="comments-prev-student-btn"
            onClick={() => selectAdjacentStudent(-1)}
            disabled={remarks.length === 0}
          >
            Previous Student
          </button>
          <button
            data-testid="comments-next-student-btn"
            onClick={() => selectAdjacentStudent(1)}
            disabled={remarks.length === 0}
          >
            Next Student
          </button>
        </div>
        {transferOpen ? (
          <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 8, marginBottom: 8 }}>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>Transfer Mode</div>
            <div style={{ display: "grid", gridTemplateColumns: "repeat(3, minmax(140px, 1fr))", gap: 6 }}>
              <label>
                Source Class
                <select
                  value={transferSourceClassId}
                  onChange={(e) => setTransferSourceClassId(e.currentTarget.value)}
                  style={{ width: "100%" }}
                >
                  {transferClasses.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Source Mark Set
                <select
                  value={transferSourceMarkSetId}
                  onChange={(e) => setTransferSourceMarkSetId(e.currentTarget.value)}
                  style={{ width: "100%" }}
                >
                  {transferSourceMarkSets.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.code}: {m.description}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Source Set
                <select
                  value={transferSourceSetNumber ?? ""}
                  onChange={(e) => setTransferSourceSetNumber(Number(e.currentTarget.value))}
                  style={{ width: "100%" }}
                >
                  {transferSourceSets.map((s) => (
                    <option key={s.setNumber} value={s.setNumber}>
                      {s.setNumber}: {s.title}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                Match
                <select
                  value={transferMatchMode}
                  onChange={(e) =>
                    setTransferMatchMode(
                      e.currentTarget.value === "name_only" ? "name_only" : "student_no_then_name"
                    )
                  }
                  style={{ width: "100%" }}
                >
                  <option value="student_no_then_name">Student # then name</option>
                  <option value="name_only">Name only</option>
                </select>
              </label>
              <label>
                Policy
                <select
                  data-testid="comments-transfer-policy"
                  value={transferPolicy}
                  onChange={(e) =>
                    setTransferPolicy(
                      e.currentTarget.value === "replace"
                        ? "replace"
                        : e.currentTarget.value === "append"
                          ? "append"
                          : e.currentTarget.value === "source_if_longer"
                            ? "source_if_longer"
                            : "fill_blank"
                    )
                  }
                  style={{ width: "100%" }}
                >
                  <option value="fill_blank">Fill Blank</option>
                  <option value="replace">Replace</option>
                  <option value="append">Append</option>
                  <option value="source_if_longer">Source If Longer</option>
                </select>
              </label>
              <label>
                Scope
                <select
                  value={transferScope}
                  onChange={(e) =>
                    setTransferScope(
                      e.currentTarget.value === "selected_target_students"
                        ? "selected_target_students"
                        : "all_matched"
                    )
                  }
                  style={{ width: "100%" }}
                >
                  <option value="all_matched">All matched</option>
                  <option value="selected_target_students">Selected targets</option>
                </select>
              </label>
              <label style={{ gridColumn: "1 / span 3" }}>
                Separator
                <input
                  value={transferSeparator}
                  onChange={(e) => setTransferSeparator(e.currentTarget.value)}
                  style={{ width: "100%" }}
                />
              </label>
            </div>
            <div style={{ display: "flex", gap: 6, marginTop: 8 }}>
              <button data-testid="comments-transfer-preview-btn" onClick={() => void runTransferPreview()}>
                Preview
              </button>
              <button data-testid="comments-transfer-apply-btn" onClick={() => void applyTransferPreview()}>
                Apply
              </button>
              <button onClick={() => setTransferOpen(false)}>Close</button>
            </div>
            {transferPreviewCounts ? (
              <div data-testid="comments-transfer-preview-summary" style={{ marginTop: 6, fontSize: 12, color: "#555" }}>
                matched {transferPreviewCounts.matched}, same {transferPreviewCounts.same}, different{" "}
                {transferPreviewCounts.different}, source-only {transferPreviewCounts.sourceOnly}, target-only{" "}
                {transferPreviewCounts.targetOnly}
              </div>
            ) : null}
            {transferPreviewRows.length > 0 ? (
              <div style={{ marginTop: 6, maxHeight: 180, overflow: "auto", border: "1px solid #eee" }}>
                {transferPreviewRows.slice(0, 120).map((row, idx) => {
                  const targetId = row.targetStudentId ?? "";
                  const checked = targetId ? transferSelectedTargetIds.includes(targetId) : false;
                  return (
                    <div key={`${targetId || "none"}-${idx}`} style={{ display: "flex", gap: 8, padding: 4, borderBottom: "1px solid #f3f3f3" }}>
                      {targetId ? (
                        <input
                          data-testid={`comments-transfer-row-select-${targetId}`}
                          type="checkbox"
                          checked={checked}
                          onChange={(e) =>
                            setTransferSelectedTargetIds((prev) => {
                              if (e.currentTarget.checked) {
                                if (prev.includes(targetId)) return prev;
                                return [...prev, targetId];
                              }
                              return prev.filter((id) => id !== targetId);
                            })
                          }
                        />
                      ) : (
                        <span style={{ width: 14 }} />
                      )}
                      <div style={{ fontSize: 12, minWidth: 80 }}>{row.status}</div>
                      <div style={{ fontSize: 12, flex: 1 }}>
                        {row.sourceDisplayName ?? "(none)"} → {row.targetDisplayName ?? "(none)"}
                      </div>
                    </div>
                  );
                })}
              </div>
            ) : null}
          </div>
        ) : null}
        {floodOpen ? (
          <div style={{ border: "1px solid #ddd", borderRadius: 8, padding: 8, marginBottom: 8 }}>
            <div style={{ fontWeight: 600, marginBottom: 4 }}>Flood Fill from Selected Student</div>
            <div style={{ color: "#666", fontSize: 12, marginBottom: 6 }}>
              Source: {selectedStudentId ? remarks.find((r) => r.studentId === selectedStudentId)?.displayName ?? selectedStudentId : "(select a student)"}
            </div>
            <button
              data-testid="comments-floodfill-apply-btn"
              disabled={!selectedStudentId}
              onClick={() => void applyFloodFillFromSelected()}
            >
              Apply Flood Fill
            </button>
          </div>
        ) : null}
        <div style={{ maxHeight: "calc(100vh - 340px)", overflow: "auto", border: "1px solid #eee" }}>
          {remarks.map((r) => (
            <div
              key={r.studentId}
              data-testid={`comment-remark-row-${r.studentId}`}
              style={{
                borderBottom: "1px solid #f0f0f0",
                padding: 8,
                background: selectedStudentId === r.studentId ? "#f9fcff" : "white"
              }}
              onClick={() => setSelectedStudentId(r.studentId)}
            >
              <div style={{ fontWeight: 600, marginBottom: 4 }}>
                {r.displayName} {!r.active ? "(inactive)" : ""}
              </div>
              <textarea
                data-testid={`comment-remark-input-${r.studentId}`}
                value={r.remark}
                onChange={(e) => {
                  const value = e.currentTarget.value;
                  setRemarks((prev) =>
                    prev.map((x) =>
                      x.studentId === r.studentId ? { ...x, remark: value } : x
                    )
                  );
                }}
                rows={3}
                style={{ width: "100%", resize: "vertical" }}
              />
            </div>
          ))}
        </div>
      </div>

      <div style={{ width: 420, borderLeft: "1px solid #ddd", paddingLeft: 12 }}>
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Comment Banks</div>
        <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
          <input
            data-testid="comments-bank-new-name"
            value={newBankShortName}
            onChange={(e) => setNewBankShortName(e.currentTarget.value)}
            placeholder="New bank short name"
            style={{ flex: 1 }}
          />
          <button data-testid="comments-bank-create-btn" onClick={() => void createBank()}>
            Create
          </button>
        </div>
        <select
          data-testid="comments-bank-select"
          value={selectedBankId ?? ""}
          onChange={(e) => setSelectedBankId(e.currentTarget.value || null)}
          style={{ width: "100%", marginBottom: 8 }}
        >
          {banks.map((b) => (
            <option key={b.id} value={b.id}>
              {b.shortName} ({b.entryCount})
            </option>
          ))}
        </select>
        {bankMeta ? (
          <div style={{ border: "1px solid #eee", padding: 8, marginBottom: 10 }}>
            <input
              data-testid="comments-bank-shortname-input"
              value={bankMeta.shortName}
              onChange={(e) => setBankMeta({ ...bankMeta, shortName: e.currentTarget.value })}
              style={{ width: "100%", marginBottom: 6 }}
            />
            <input
              data-testid="comments-bank-fitprofile-input"
              value={bankMeta.fitProfile ?? ""}
              onChange={(e) => setBankMeta({ ...bankMeta, fitProfile: e.currentTarget.value })}
              placeholder="Fit profile"
              style={{ width: "100%", marginBottom: 6 }}
            />
            <label style={{ display: "block", marginBottom: 6 }}>
              <input
                type="checkbox"
                checked={bankMeta.isDefault}
                onChange={(e) => setBankMeta({ ...bankMeta, isDefault: e.currentTarget.checked })}
              />{" "}
              Default bank
            </label>
            <button data-testid="comments-bank-save-meta-btn" onClick={() => void saveBankMeta()}>
              Save Bank Meta
            </button>
          </div>
        ) : null}
        <div style={{ display: "flex", gap: 6, marginBottom: 8 }}>
          <button data-testid="comments-bank-entry-add-btn" onClick={() => void addEntry()}>
            Add Entry
          </button>
          <select
            data-testid="comments-bank-apply-mode-select"
            value={applyMode}
            onChange={(e) => setApplyMode(e.currentTarget.value as "append" | "replace")}
          >
            <option value="append">Append</option>
            <option value="replace">Replace</option>
          </select>
          <button
            data-testid="comments-bank-apply-btn"
            disabled={!selectedStudentId || !selectedBankEntry}
            onClick={() => applySelectedBankEntryToStudent()}
          >
            Apply Selected Entry
          </button>
        </div>
        <div style={{ maxHeight: 260, overflow: "auto", border: "1px solid #eee", marginBottom: 10 }}>
          {bankEntries.map((e) => (
            <div
              key={e.id}
              data-testid={`comments-bank-entry-row-${e.id}`}
              style={{
                borderBottom: "1px solid #f3f3f3",
                padding: 8,
                background: selectedBankEntry?.id === e.id ? "#f9fcff" : "white",
                cursor: "pointer"
              }}
              onClick={() => setSelectedBankEntryId(e.id)}
            >
              <div style={{ display: "flex", gap: 6, marginBottom: 4 }}>
                <input
                  value={e.typeCode}
                  onChange={(ev) => {
                    const value = ev.currentTarget.value;
                    setBankEntries((prev) =>
                      prev.map((x) => (x.id === e.id ? { ...x, typeCode: value } : x))
                    );
                  }}
                  style={{ width: 64 }}
                />
                <input
                  value={e.levelCode}
                  onChange={(ev) => {
                    const value = ev.currentTarget.value;
                    setBankEntries((prev) =>
                      prev.map((x) => (x.id === e.id ? { ...x, levelCode: value } : x))
                    );
                  }}
                  style={{ width: 64 }}
                />
                <input
                  type="number"
                  value={e.sortOrder}
                  onChange={(ev) => {
                    const value = Number(ev.currentTarget.value) || 0;
                    setBankEntries((prev) =>
                      prev.map((x) =>
                        x.id === e.id ? { ...x, sortOrder: value } : x
                      )
                    );
                  }}
                  style={{ width: 64 }}
                />
                <button onClick={() => void updateEntry(e)}>Save</button>
                <button
                  data-testid={`comments-bank-entry-delete-${e.id}`}
                  onClick={() => void deleteEntry(e.id)}
                >
                  Delete
                </button>
              </div>
              <textarea
                value={e.text}
                onChange={(ev) => {
                  const value = ev.currentTarget.value;
                  setBankEntries((prev) =>
                    prev.map((x) => (x.id === e.id ? { ...x, text: value } : x))
                  );
                }}
                rows={3}
                style={{ width: "100%" }}
              />
            </div>
          ))}
        </div>

        <div style={{ border: "1px solid #eee", padding: 8 }}>
          <div style={{ fontWeight: 600, marginBottom: 6 }}>Import/Export .BNK</div>
          <div style={{ display: "flex", gap: 6, marginBottom: 6 }}>
            <input
              data-testid="comments-bank-import-path"
              value={importPath}
              onChange={(e) => setImportPath(e.currentTarget.value)}
              placeholder="Path to .BNK"
              style={{ flex: 1 }}
            />
            <button data-testid="comments-bank-import-browse-btn" onClick={() => void browseImportBnkPath()}>
              Browse
            </button>
          </div>
          <button data-testid="comments-bank-import-btn" onClick={() => void importBnk()}>
            Import BNK
          </button>
          <div style={{ display: "flex", gap: 6, margin: "8px 0 6px" }}>
            <input
              data-testid="comments-bank-export-path"
              value={exportPath}
              onChange={(e) => setExportPath(e.currentTarget.value)}
              placeholder="Export path"
              style={{ flex: 1 }}
            />
            <button data-testid="comments-bank-export-browse-btn" onClick={() => void browseExportBnkPath()}>
              Browse
            </button>
          </div>
          <button data-testid="comments-bank-export-btn" onClick={() => void exportBnk()}>
            Export BNK
          </button>
        </div>
      </div>
    </div>
  );
}
