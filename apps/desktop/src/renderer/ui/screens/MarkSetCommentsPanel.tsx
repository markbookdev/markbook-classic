import React, { useEffect, useMemo, useState } from "react";
import {
  CommentsBanksCreateResultSchema,
  CommentsBanksEntryDeleteResultSchema,
  CommentsBanksEntryUpsertResultSchema,
  CommentsBanksExportBnkResultSchema,
  CommentsBanksImportBnkResultSchema,
  CommentsBanksListResultSchema,
  CommentsBanksOpenResultSchema,
  CommentsBanksUpdateMetaResultSchema,
  CommentsSetsDeleteResultSchema,
  CommentsSetsListResultSchema,
  CommentsSetsOpenResultSchema,
  CommentsSetsUpsertResultSchema
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

  const selectedStudentRemark = useMemo(
    () => remarks.find((r) => r.studentId === selectedStudentId)?.remark ?? "",
    [remarks, selectedStudentId]
  );

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
    setBankEntries(res.entries as BankEntry[]);
  }

  async function refreshAll() {
    props.onError(null);
    try {
      await Promise.all([loadSets(), loadBanks()]);
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
      const res = await requestParsed(
        "comments.sets.upsert",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          title,
          fitMode: 0,
          fitFontSize: 9,
          fitWidth: 83,
          fitLines: 12,
          fitSubj: "",
          maxChars: 100,
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
          <button
            data-testid="comments-bank-apply-btn"
            disabled={!selectedStudentId}
            onClick={() => {
              const first = bankEntries[0];
              if (!first || !selectedStudentId) return;
              setRemarks((prev) =>
                prev.map((r) =>
                  r.studentId === selectedStudentId
                    ? { ...r, remark: (selectedStudentRemark + " " + first.text).trim() }
                    : r
                )
              );
            }}
          >
            Apply First Entry
          </button>
        </div>
        <div style={{ maxHeight: 260, overflow: "auto", border: "1px solid #eee", marginBottom: 10 }}>
          {bankEntries.map((e) => (
            <div key={e.id} style={{ borderBottom: "1px solid #f3f3f3", padding: 8 }}>
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
          <input
            data-testid="comments-bank-import-path"
            value={importPath}
            onChange={(e) => setImportPath(e.currentTarget.value)}
            placeholder="Path to .BNK"
            style={{ width: "100%", marginBottom: 6 }}
          />
          <button data-testid="comments-bank-import-btn" onClick={() => void importBnk()}>
            Import BNK
          </button>
          <input
            data-testid="comments-bank-export-path"
            value={exportPath}
            onChange={(e) => setExportPath(e.currentTarget.value)}
            placeholder="Export path"
            style={{ width: "100%", margin: "8px 0 6px" }}
          />
          <button data-testid="comments-bank-export-btn" onClick={() => void exportBnk()}>
            Export BNK
          </button>
        </div>
      </div>
    </div>
  );
}
