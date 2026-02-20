import React, { useEffect, useMemo, useState } from "react";
import {
  AssessmentsCreateResultSchema,
  AssessmentsDeleteResultSchema,
  AssessmentsListResultSchema,
  AssessmentsReorderResultSchema,
  AssessmentsUpdateResultSchema,
  ClassesListResultSchema,
  CategoriesCreateResultSchema,
  CategoriesDeleteResultSchema,
  CategoriesListResultSchema,
  CategoriesUpdateResultSchema,
  MarkSetsCloneResultSchema,
  MarkSetsCreateResultSchema,
  MarkSetsDeleteResultSchema,
  MarkSetsListResultSchema,
  MarkSetsSetDefaultResultSchema,
  MarkSetsTransferApplyResultSchema,
  MarkSetsTransferPreviewResultSchema,
  MarkSetsUndeleteResultSchema,
  MarkSetSettingsGetResultSchema,
  MarkSetSettingsUpdateResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";
import { MarkSetCommentsPanel } from "./MarkSetCommentsPanel";

type CategoryRow = {
  id: string;
  name: string;
  weight: number | null;
  sortOrder: number;
};

type AssessmentRow = {
  id: string;
  idx: number;
  date: string | null;
  categoryName: string | null;
  title: string;
  term: number | null;
  legacyType: number | null;
  weight: number | null;
  outOf: number | null;
};

type MarkSetManagerRow = {
  id: string;
  code: string;
  description: string;
  sortOrder: number;
  isDefault?: boolean;
  deletedAt?: string | null;
};

type TransferClassRow = {
  id: string;
  name: string;
};

type TransferPreview = {
  sourceAssessmentCount: number;
  candidateCount: number;
  collisions: Array<{
    sourceAssessmentId: string;
    sourceIdx: number;
    sourceTitle: string;
    targetAssessmentId: string;
    targetIdx: number;
    key: string;
  }>;
  studentAlignment: {
    sourceRows: number;
    targetRows: number;
    alignedRows: number;
  };
};

function parseNullableNumber(s: string): number | null {
  const t = s.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return n;
}

function parseNullableInt(s: string): number | null {
  const t = s.trim();
  if (!t) return null;
  const n = Number(t);
  if (!Number.isFinite(n)) return null;
  return Math.trunc(n);
}

function suggestCloneCode(code: string): string {
  const trimmed = code.trim();
  const match = trimmed.match(/^(.*?)(\d+)$/);
  if (!match) return `${trimmed}C`.slice(0, 15);
  const prefix = match[1];
  const suffix = match[2];
  const width = suffix.length;
  const next = String(Number(suffix) + 1).padStart(width, "0");
  return `${prefix}${next}`.slice(0, 15);
}

export function MarkSetSetupScreen(props: {
  selectedClassId: string;
  selectedMarkSetId: string;
  onError: (msg: string | null) => void;
  onChanged?: () => void | Promise<void>;
  onSelectMarkSet?: (markSetId: string) => void;
}) {
  const [tab, setTab] = useState<"setup" | "comments">("setup");
  const [loading, setLoading] = useState(false);
  const [categories, setCategories] = useState<CategoryRow[]>([]);
  const [assessments, setAssessments] = useState<AssessmentRow[]>([]);
  const [fullCode, setFullCode] = useState("");
  const [room, setRoom] = useState("");
  const [day, setDay] = useState("");
  const [period, setPeriod] = useState("");
  const [weightMethod, setWeightMethod] = useState("1");
  const [calcMethod, setCalcMethod] = useState("0");
  const [blockTitle, setBlockTitle] = useState("");
  const [markSetRows, setMarkSetRows] = useState<MarkSetManagerRow[]>([]);
  const [managerIncludeDeleted, setManagerIncludeDeleted] = useState(true);
  const [managerSearch, setManagerSearch] = useState("");
  const [managerStatusFilter, setManagerStatusFilter] = useState<"all" | "active" | "deleted">(
    "all"
  );
  const [managerSort, setManagerSort] = useState<"sort" | "code" | "description">("sort");
  const [newMarkSetCode, setNewMarkSetCode] = useState("");
  const [newMarkSetDescription, setNewMarkSetDescription] = useState("");
  const [newMarkSetBlockTitle, setNewMarkSetBlockTitle] = useState("");
  const [newMarkSetWeightMethod, setNewMarkSetWeightMethod] = useState("1");
  const [newMarkSetCalcMethod, setNewMarkSetCalcMethod] = useState("0");
  const [cloneModal, setCloneModal] = useState<{
    source: MarkSetManagerRow;
    code: string;
    description: string;
    cloneAssessments: boolean;
    cloneScores: boolean;
  } | null>(null);
  const [transferOpen, setTransferOpen] = useState(false);
  const [transferClasses, setTransferClasses] = useState<TransferClassRow[]>([]);
  const [transferSourceClassId, setTransferSourceClassId] = useState("");
  const [transferSourceMarkSetId, setTransferSourceMarkSetId] = useState("");
  const [transferSourceMarkSets, setTransferSourceMarkSets] = useState<MarkSetManagerRow[]>([]);
  const [transferCollisionPolicy, setTransferCollisionPolicy] = useState<
    "merge_existing" | "append_new" | "stop_on_collision"
  >("merge_existing");
  const [transferTitleMode, setTransferTitleMode] = useState<"same" | "appendTransfer">("same");
  const [transferPreview, setTransferPreview] = useState<TransferPreview | null>(null);

  const [newCategoryName, setNewCategoryName] = useState("");
  const [newCategoryWeight, setNewCategoryWeight] = useState("20");

  const [newTitle, setNewTitle] = useState("");
  const [newDate, setNewDate] = useState("");
  const [newCategoryName2, setNewCategoryName2] = useState("");
  const [newTerm, setNewTerm] = useState("1");
  const [newWeight, setNewWeight] = useState("1");
  const [newOutOf, setNewOutOf] = useState("10");

  const canAddCategory = useMemo(() => newCategoryName.trim().length > 0, [newCategoryName]);
  const canAddAssessment = useMemo(() => newTitle.trim().length > 0, [newTitle]);
  const canCreateMarkSet = useMemo(
    () => newMarkSetCode.trim().length > 0 && newMarkSetDescription.trim().length > 0,
    [newMarkSetCode, newMarkSetDescription]
  );

  async function loadMarkSetManager() {
    props.onError(null);
    try {
      const list = await requestParsed(
        "marksets.list",
        { classId: props.selectedClassId, includeDeleted: managerIncludeDeleted },
        MarkSetsListResultSchema
      );
      setMarkSetRows(list.markSets as any);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setMarkSetRows([]);
    }
  }

  async function loadAll() {
    setLoading(true);
    props.onError(null);
    try {
      const [cats, asmt, settings, list] = await Promise.all([
        requestParsed(
          "categories.list",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          CategoriesListResultSchema
        ),
        requestParsed(
          "assessments.list",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          AssessmentsListResultSchema
        ),
        requestParsed(
          "markset.settings.get",
          { classId: props.selectedClassId, markSetId: props.selectedMarkSetId },
          MarkSetSettingsGetResultSchema
        ),
        requestParsed(
          "marksets.list",
          { classId: props.selectedClassId, includeDeleted: managerIncludeDeleted },
          MarkSetsListResultSchema
        )
      ]);
      setCategories(cats.categories);
      setAssessments(asmt.assessments);
      setMarkSetRows(list.markSets as any);
      setFullCode(settings.markSet.fullCode ?? "");
      setRoom(settings.markSet.room ?? "");
      setDay(settings.markSet.day ?? "");
      setPeriod(settings.markSet.period ?? "");
      setBlockTitle(settings.markSet.blockTitle ?? "");
      setWeightMethod(String(settings.markSet.weightMethod ?? 1));
      setCalcMethod(String(settings.markSet.calcMethod ?? 0));
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setCategories([]);
      setAssessments([]);
      setFullCode("");
      setRoom("");
      setDay("");
      setPeriod("");
      setBlockTitle("");
      setWeightMethod("1");
      setCalcMethod("0");
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, props.selectedMarkSetId, managerIncludeDeleted]);

  async function saveMarkSetSettings() {
    props.onError(null);
    try {
      await requestParsed(
        "markset.settings.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          patch: {
            fullCode: fullCode.trim() || null,
            room: room.trim() || null,
            day: day.trim() || null,
            period: period.trim() || null,
            blockTitle: blockTitle.trim() || null,
            weightMethod: parseNullableInt(weightMethod) ?? 1,
            calcMethod: parseNullableInt(calcMethod) ?? 0
          }
        },
        MarkSetSettingsUpdateResultSchema
      );
      await props.onChanged?.();
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function updateCategoryLocal(id: string, patch: Partial<CategoryRow>) {
    setCategories((prev) => prev.map((c) => (c.id === id ? { ...c, ...patch } : c)));
  }
  async function updateCategory(
    categoryId: string,
    patch: { name?: string; weight?: number | null }
  ) {
    props.onError(null);
    try {
      await requestParsed(
        "categories.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          categoryId,
          patch
        },
        CategoriesUpdateResultSchema
      );
      updateCategoryLocal(categoryId, patch as any);
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  async function addCategory() {
    if (!canAddCategory) return;
    props.onError(null);
    try {
      await requestParsed(
        "categories.create",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          name: newCategoryName.trim(),
          weight: parseNullableNumber(newCategoryWeight) ?? 0
        },
        CategoriesCreateResultSchema
      );
      setNewCategoryName("");
      setNewCategoryWeight("20");
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteCategory(categoryId: string) {
    const ok = confirm("Delete this category?");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "categories.delete",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          categoryId
        },
        CategoriesDeleteResultSchema
      );
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function updateAssessmentLocal(id: string, patch: Partial<AssessmentRow>) {
    setAssessments((prev) => prev.map((a) => (a.id === id ? { ...a, ...patch } : a)));
  }

  async function updateAssessment(
    assessmentId: string,
    patch: {
      date?: string | null;
      categoryName?: string | null;
      title?: string;
      term?: number | null;
      legacyType?: number | null;
      weight?: number | null;
      outOf?: number | null;
    }
  ) {
    props.onError(null);
    try {
      await requestParsed(
        "assessments.update",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          assessmentId,
          patch
        },
        AssessmentsUpdateResultSchema
      );
      updateAssessmentLocal(assessmentId, patch as any);
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  async function addAssessment() {
    if (!canAddAssessment) return;
    props.onError(null);
    try {
      await requestParsed(
        "assessments.create",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          title: newTitle.trim(),
          date: newDate.trim() ? newDate.trim() : null,
          categoryName: newCategoryName2.trim() ? newCategoryName2.trim() : null,
          term: parseNullableInt(newTerm),
          weight: parseNullableNumber(newWeight),
          outOf: parseNullableNumber(newOutOf)
        },
        AssessmentsCreateResultSchema
      );
      setNewTitle("");
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function createMarkSet() {
    if (!canCreateMarkSet) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "marksets.create",
        {
          classId: props.selectedClassId,
          code: newMarkSetCode.trim(),
          description: newMarkSetDescription.trim(),
          blockTitle: newMarkSetBlockTitle.trim() || null,
          weightMethod: parseNullableInt(newMarkSetWeightMethod) ?? 1,
          calcMethod: parseNullableInt(newMarkSetCalcMethod) ?? 0,
          starterCategories: [{ name: "Category 1", weight: 100 }]
        },
        MarkSetsCreateResultSchema
      );
      setNewMarkSetCode("");
      setNewMarkSetDescription("");
      setNewMarkSetBlockTitle("");
      setNewMarkSetWeightMethod("1");
      setNewMarkSetCalcMethod("0");
      props.onSelectMarkSet?.(res.markSetId);
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function deleteMarkSet(markSetId: string) {
    const ok = confirm("Delete this mark set? It will be soft-deleted and can be undeleted later.");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "marksets.delete",
        { classId: props.selectedClassId, markSetId },
        MarkSetsDeleteResultSchema
      );
      await loadMarkSetManager();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function undeleteMarkSet(markSetId: string) {
    props.onError(null);
    try {
      await requestParsed(
        "marksets.undelete",
        { classId: props.selectedClassId, markSetId },
        MarkSetsUndeleteResultSchema
      );
      props.onSelectMarkSet?.(markSetId);
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function setDefaultMarkSet(markSetId: string) {
    props.onError(null);
    try {
      await requestParsed(
        "marksets.setDefault",
        { classId: props.selectedClassId, markSetId },
        MarkSetsSetDefaultResultSchema
      );
      await loadMarkSetManager();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  function openCloneMarkSet(markSet: MarkSetManagerRow) {
    const nextCode = suggestCloneCode(markSet.code);
    setCloneModal({
      source: markSet,
      code: nextCode,
      description: `${markSet.description} (Copy)`,
      cloneAssessments: true,
      cloneScores: false
    });
  }

  async function submitCloneMarkSet() {
    if (!cloneModal) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "marksets.clone",
        {
          classId: props.selectedClassId,
          markSetId: cloneModal.source.id,
          code: cloneModal.code.trim(),
          description: cloneModal.description.trim(),
          cloneAssessments: cloneModal.cloneAssessments,
          cloneScores: cloneModal.cloneScores
        },
        MarkSetsCloneResultSchema
      );
      setCloneModal(null);
      props.onSelectMarkSet?.(res.markSetId);
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function loadTransferSourceMarkSets(sourceClassId: string) {
    if (!sourceClassId) {
      setTransferSourceMarkSets([]);
      setTransferSourceMarkSetId("");
      return;
    }
    try {
      const list = await requestParsed(
        "marksets.list",
        { classId: sourceClassId, includeDeleted: false },
        MarkSetsListResultSchema
      );
      setTransferSourceMarkSets(list.markSets as any);
      setTransferSourceMarkSetId((cur) => {
        if (cur && list.markSets.some((m) => m.id === cur)) return cur;
        const fallback =
          list.markSets.find((m) => m.id !== props.selectedMarkSetId)?.id ??
          list.markSets[0]?.id ??
          "";
        return fallback;
      });
    } catch {
      setTransferSourceMarkSets([]);
      setTransferSourceMarkSetId("");
    }
  }

  async function openTransferDialog() {
    props.onError(null);
    try {
      const cls = await requestParsed("classes.list", {}, ClassesListResultSchema);
      const rows = cls.classes.map((c) => ({ id: c.id, name: c.name }));
      setTransferClasses(rows);
      const preferredSource =
        rows.find((c) => c.id !== props.selectedClassId)?.id ?? props.selectedClassId;
      setTransferSourceClassId(preferredSource);
      setTransferCollisionPolicy("merge_existing");
      setTransferTitleMode("same");
      setTransferPreview(null);
      setTransferOpen(true);
      await loadTransferSourceMarkSets(preferredSource);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function previewTransfer() {
    if (!transferSourceClassId || !transferSourceMarkSetId) return;
    props.onError(null);
    try {
      const res = await requestParsed(
        "marksets.transfer.preview",
        {
          sourceClassId: transferSourceClassId,
          sourceMarkSetId: transferSourceMarkSetId,
          targetClassId: props.selectedClassId,
          targetMarkSetId: props.selectedMarkSetId
        },
        MarkSetsTransferPreviewResultSchema
      );
      setTransferPreview(res as TransferPreview);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      setTransferPreview(null);
    }
  }

  async function applyTransfer() {
    if (!transferSourceClassId || !transferSourceMarkSetId) return;
    props.onError(null);
    try {
      await requestParsed(
        "marksets.transfer.apply",
        {
          sourceClassId: transferSourceClassId,
          sourceMarkSetId: transferSourceMarkSetId,
          targetClassId: props.selectedClassId,
          targetMarkSetId: props.selectedMarkSetId,
          collisionPolicy: transferCollisionPolicy,
          titleMode: transferTitleMode
        },
        MarkSetsTransferApplyResultSchema
      );
      setTransferOpen(false);
      setTransferPreview(null);
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  const visibleMarkSets = useMemo(() => {
    const needle = managerSearch.trim().toLowerCase();
    const rows = markSetRows.filter((row) => {
      const deleted = Boolean(row.deletedAt);
      if (managerStatusFilter === "active" && deleted) return false;
      if (managerStatusFilter === "deleted" && !deleted) return false;
      if (!needle) return true;
      return (
        row.code.toLowerCase().includes(needle) ||
        row.description.toLowerCase().includes(needle)
      );
    });
    const out = rows.slice();
    out.sort((a, b) => {
      if (managerSort === "code") return a.code.localeCompare(b.code);
      if (managerSort === "description") return a.description.localeCompare(b.description);
      return a.sortOrder - b.sortOrder;
    });
    return out;
  }, [markSetRows, managerSearch, managerSort, managerStatusFilter]);

  async function deleteAssessment(assessmentId: string) {
    const ok = confirm("Delete this assessment? All related marks will be removed.");
    if (!ok) return;
    props.onError(null);
    try {
      await requestParsed(
        "assessments.delete",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          assessmentId
        },
        AssessmentsDeleteResultSchema
      );
      await loadAll();
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function moveAssessment(idx: number, dir: -1 | 1) {
    const nextIdx = idx + dir;
    if (nextIdx < 0 || nextIdx >= assessments.length) return;
    const next = assessments.slice();
    const [row] = next.splice(idx, 1);
    next.splice(nextIdx, 0, row);
    const orderedAssessmentIds = next.map((a) => a.id);

    props.onError(null);
    try {
      await requestParsed(
        "assessments.reorder",
        {
          classId: props.selectedClassId,
          markSetId: props.selectedMarkSetId,
          orderedAssessmentIds
        },
        AssessmentsReorderResultSchema
      );
      setAssessments(next.map((a, i) => ({ ...a, idx: i })));
      await props.onChanged?.();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
      await loadAll();
    }
  }

  const inputStyle: React.CSSProperties = useMemo(
    () => ({
      width: "100%",
      padding: "6px 8px",
      border: "1px solid #ddd",
      borderRadius: 6
    }),
    []
  );

  return (
    <div data-testid="markset-setup-screen" style={{ padding: 24, maxWidth: 1200 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Mark Set Setup</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button
          data-testid="markset-setup-tab-setup"
          onClick={() => setTab("setup")}
          style={{ fontWeight: tab === "setup" ? 700 : 400 }}
        >
          Setup
        </button>
        <button
          data-testid="markset-setup-tab-comments"
          onClick={() => setTab("comments")}
          style={{ fontWeight: tab === "comments" ? 700 : 400 }}
        >
          Comments
        </button>
      </div>

      {tab === "setup" ? (
        <>

      <div style={{ display: "flex", gap: 16, alignItems: "center", marginBottom: 10 }}>
        <div style={{ color: "#555", fontSize: 13 }}>
          {loading
            ? "Loading..."
            : `${categories.length} categories, ${assessments.length} assessments`}
        </div>
        <button onClick={() => void loadAll()} disabled={loading}>
          Reload
        </button>
      </div>

      <div
        data-testid="markset-manager-panel"
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          marginBottom: 16
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Mark Set Manager</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 10 }}>
          <label
            style={{
              display: "flex",
              alignItems: "center",
              gap: 6,
              fontSize: 12,
              color: "#444"
            }}
          >
            <input
              data-testid="markset-manager-include-deleted"
              type="checkbox"
              checked={managerIncludeDeleted}
              onChange={(e) => setManagerIncludeDeleted(e.currentTarget.checked)}
            />
            Show deleted
          </label>
          <input
            data-testid="markset-manager-search"
            value={managerSearch}
            onChange={(e) => setManagerSearch(e.currentTarget.value)}
            placeholder="Search code or description"
            style={{ ...inputStyle, width: 220 }}
          />
          <select
            data-testid="markset-manager-status-filter"
            value={managerStatusFilter}
            onChange={(e) =>
              setManagerStatusFilter((e.currentTarget.value as any) || "all")
            }
            style={{ ...inputStyle, width: 140 }}
          >
            <option value="all">All statuses</option>
            <option value="active">Active only</option>
            <option value="deleted">Deleted only</option>
          </select>
          <select
            data-testid="markset-manager-sort"
            value={managerSort}
            onChange={(e) => setManagerSort((e.currentTarget.value as any) || "sort")}
            style={{ ...inputStyle, width: 150 }}
          >
            <option value="sort">Sort: legacy order</option>
            <option value="code">Sort: code</option>
            <option value="description">Sort: description</option>
          </select>
          <div style={{ color: "#666", fontSize: 12, alignSelf: "center" }}>
            {visibleMarkSets.length} shown / {markSetRows.length} total
          </div>
          <button data-testid="markset-transfer-open-btn" onClick={() => void openTransferDialog()}>
            Transfer From Mark Set
          </button>
        </div>

        <div style={{ display: "flex", gap: 8, flexWrap: "wrap", marginBottom: 12 }}>
          <input
            data-testid="markset-manager-new-code"
            value={newMarkSetCode}
            onChange={(e) => setNewMarkSetCode(e.currentTarget.value)}
            placeholder="Short code"
            style={{ ...inputStyle, width: 130 }}
          />
          <input
            data-testid="markset-manager-new-description"
            value={newMarkSetDescription}
            onChange={(e) => setNewMarkSetDescription(e.currentTarget.value)}
            placeholder="Description"
            style={{ ...inputStyle, flex: "1 1 220px" }}
          />
          <input
            data-testid="markset-manager-new-block-title"
            value={newMarkSetBlockTitle}
            onChange={(e) => setNewMarkSetBlockTitle(e.currentTarget.value)}
            placeholder="Block title"
            style={{ ...inputStyle, width: 140 }}
          />
          <select
            data-testid="markset-manager-new-weight-method"
            value={newMarkSetWeightMethod}
            onChange={(e) => setNewMarkSetWeightMethod(e.currentTarget.value)}
            style={{ ...inputStyle, width: 140 }}
          >
            <option value="0">Entry</option>
            <option value="1">Category</option>
            <option value="2">Equal</option>
          </select>
          <select
            data-testid="markset-manager-new-calc-method"
            value={newMarkSetCalcMethod}
            onChange={(e) => setNewMarkSetCalcMethod(e.currentTarget.value)}
            style={{ ...inputStyle, width: 140 }}
          >
            <option value="0">Average</option>
            <option value="1">Median</option>
            <option value="2">Mode</option>
            <option value="3">Blend Mode</option>
            <option value="4">Blend Median</option>
          </select>
          <button
            data-testid="markset-manager-create-btn"
            disabled={!canCreateMarkSet}
            onClick={() => void createMarkSet()}
          >
            Create Mark Set
          </button>
        </div>
        <div
          data-testid="markset-manager-table-wrap"
          style={{ border: "1px solid #eee", borderRadius: 8, overflow: "auto" }}
        >
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
                <th style={{ textAlign: "left", padding: 8, width: 90 }}>Code</th>
                <th style={{ textAlign: "left", padding: 8 }}>Description</th>
                <th style={{ textAlign: "left", padding: 8, width: 110 }}>Status</th>
                <th style={{ textAlign: "left", padding: 8, width: 320 }}>Actions</th>
              </tr>
            </thead>
            <tbody>
              {visibleMarkSets.map((ms) => {
                const deleted = Boolean(ms.deletedAt);
                return (
                  <tr key={ms.id} style={{ borderBottom: "1px solid #f0f0f0" }}>
                    <td style={{ padding: 8 }}>{ms.code}</td>
                    <td style={{ padding: 8 }}>{ms.description}</td>
                    <td style={{ padding: 8, color: deleted ? "#8a1f11" : "#2e7d32" }}>
                      {deleted ? "Deleted" : ms.isDefault ? "Default" : "Active"}
                    </td>
                    <td style={{ padding: 8 }}>
                      <div style={{ display: "flex", gap: 6, flexWrap: "wrap" }}>
                        <button
                          data-testid={`markset-manager-open-${ms.id}`}
                          disabled={deleted}
                          onClick={() => props.onSelectMarkSet?.(ms.id)}
                        >
                          Open
                        </button>
                        <button
                          data-testid={`markset-manager-clone-${ms.id}`}
                          disabled={deleted}
                          onClick={() => openCloneMarkSet(ms)}
                        >
                          Clone
                        </button>
                        <button
                          data-testid={`markset-manager-default-${ms.id}`}
                          disabled={deleted || Boolean(ms.isDefault)}
                          onClick={() => void setDefaultMarkSet(ms.id)}
                        >
                          Set Default
                        </button>
                        {deleted ? (
                          <button
                            data-testid={`markset-manager-undelete-${ms.id}`}
                            onClick={() => void undeleteMarkSet(ms.id)}
                          >
                            Undelete
                          </button>
                        ) : (
                          <button
                            data-testid={`markset-manager-delete-${ms.id}`}
                            style={{ color: "#b00020" }}
                            onClick={() => void deleteMarkSet(ms.id)}
                          >
                            Delete
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {cloneModal ? (
        <div
          data-testid="markset-clone-modal"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.2)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 50
          }}
        >
          <div
            style={{
              width: 520,
              background: "#fff",
              borderRadius: 10,
              border: "1px solid #ddd",
              padding: 16
            }}
          >
            <div style={{ fontWeight: 700, marginBottom: 8 }}>Clone Mark Set</div>
            <div style={{ color: "#555", marginBottom: 10, fontSize: 13 }}>
              Source: {cloneModal.source.code} - {cloneModal.source.description}
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                New code
                <input
                  data-testid="markset-clone-code"
                  value={cloneModal.code}
                  onChange={(e) =>
                    setCloneModal((cur) => (cur ? { ...cur, code: e.currentTarget.value } : cur))
                  }
                  style={inputStyle}
                />
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                Description
                <input
                  data-testid="markset-clone-description"
                  value={cloneModal.description}
                  onChange={(e) =>
                    setCloneModal((cur) =>
                      cur ? { ...cur, description: e.currentTarget.value } : cur
                    )
                  }
                  style={inputStyle}
                />
              </label>
            </div>
            <div style={{ display: "flex", gap: 12, marginTop: 10 }}>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input
                  data-testid="markset-clone-with-assessments"
                  type="checkbox"
                  checked={cloneModal.cloneAssessments}
                  onChange={(e) =>
                    setCloneModal((cur) =>
                      cur
                        ? {
                            ...cur,
                            cloneAssessments: e.currentTarget.checked,
                            cloneScores: e.currentTarget.checked ? cur.cloneScores : false
                          }
                        : cur
                    )
                  }
                />
                Clone assessments
              </label>
              <label style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <input
                  data-testid="markset-clone-with-scores"
                  type="checkbox"
                  checked={cloneModal.cloneScores}
                  disabled={!cloneModal.cloneAssessments}
                  onChange={(e) =>
                    setCloneModal((cur) =>
                      cur ? { ...cur, cloneScores: e.currentTarget.checked } : cur
                    )
                  }
                />
                Clone scores
              </label>
            </div>
            <div style={{ display: "flex", gap: 8, marginTop: 14 }}>
              <button
                data-testid="markset-clone-confirm-btn"
                onClick={() => void submitCloneMarkSet()}
              >
                Clone
              </button>
              <button
                data-testid="markset-clone-cancel-btn"
                onClick={() => setCloneModal(null)}
              >
                Cancel
              </button>
            </div>
          </div>
        </div>
      ) : null}

      {transferOpen ? (
        <div
          data-testid="markset-transfer-modal"
          style={{
            position: "fixed",
            inset: 0,
            background: "rgba(0,0,0,0.2)",
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            zIndex: 50
          }}
        >
          <div
            style={{
              width: 760,
              maxWidth: "96vw",
              maxHeight: "90vh",
              overflow: "auto",
              background: "#fff",
              borderRadius: 10,
              border: "1px solid #ddd",
              padding: 16
            }}
          >
            <div style={{ fontWeight: 700, marginBottom: 8 }}>Transfer Mark Set Content</div>
            <div style={{ color: "#666", fontSize: 12, marginBottom: 12 }}>
              Target: current class and selected mark set.
            </div>

            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                Source class
                <select
                  value={transferSourceClassId}
                  onChange={(e) => {
                    const next = e.currentTarget.value;
                    setTransferSourceClassId(next);
                    setTransferPreview(null);
                    void loadTransferSourceMarkSets(next);
                  }}
                  style={inputStyle}
                >
                  {transferClasses.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                Source mark set
                <select
                  value={transferSourceMarkSetId}
                  onChange={(e) => {
                    setTransferSourceMarkSetId(e.currentTarget.value);
                    setTransferPreview(null);
                  }}
                  style={inputStyle}
                >
                  {transferSourceMarkSets.map((m) => (
                    <option key={m.id} value={m.id}>
                      {m.code}: {m.description}
                    </option>
                  ))}
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                Collision policy
                <select
                  value={transferCollisionPolicy}
                  onChange={(e) =>
                    setTransferCollisionPolicy(
                      (e.currentTarget.value as
                        | "merge_existing"
                        | "append_new"
                        | "stop_on_collision") ?? "merge_existing"
                    )
                  }
                  style={inputStyle}
                >
                  <option value="merge_existing">Merge existing (default)</option>
                  <option value="append_new">Append new</option>
                  <option value="stop_on_collision">Stop on collision</option>
                </select>
              </label>
              <label style={{ display: "flex", flexDirection: "column", gap: 4 }}>
                Title mode
                <select
                  value={transferTitleMode}
                  onChange={(e) =>
                    setTransferTitleMode(
                      (e.currentTarget.value as "same" | "appendTransfer") ?? "same"
                    )
                  }
                  style={inputStyle}
                >
                  <option value="same">Keep titles</option>
                  <option value="appendTransfer">Append "(Transfer)"</option>
                </select>
              </label>
            </div>

            <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
              <button data-testid="markset-transfer-preview-btn" onClick={() => void previewTransfer()}>
                Preview
              </button>
              <button
                data-testid="markset-transfer-apply-btn"
                onClick={() => void applyTransfer()}
                disabled={!transferPreview}
              >
                Apply Transfer
              </button>
              <button
                onClick={() => {
                  setTransferOpen(false);
                  setTransferPreview(null);
                }}
              >
                Cancel
              </button>
            </div>

            {transferPreview ? (
              <div
                data-testid="markset-transfer-preview-summary"
                style={{
                  marginTop: 12,
                  border: "1px solid #eee",
                  borderRadius: 8,
                  background: "#fafafa",
                  padding: 10,
                  fontSize: 12
                }}
              >
                <div>
                  Source assessments: {transferPreview.sourceAssessmentCount}
                </div>
                <div>Candidate assessments: {transferPreview.candidateCount}</div>
                <div>Collisions: {transferPreview.collisions.length}</div>
                <div>
                  Student alignment: source {transferPreview.studentAlignment.sourceRows}, target{" "}
                  {transferPreview.studentAlignment.targetRows}, aligned{" "}
                  {transferPreview.studentAlignment.alignedRows}
                </div>
              </div>
            ) : null}
          </div>
        </div>
      ) : null}

      <div
        style={{
          border: "1px solid #ddd",
          borderRadius: 10,
          padding: 16,
          marginBottom: 16
        }}
      >
        <div style={{ fontWeight: 700, marginBottom: 8 }}>Mark Set Settings</div>
        <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
          <input
            data-testid="markset-fullcode-input"
            value={fullCode}
            onChange={(e) => setFullCode(e.currentTarget.value)}
            placeholder="Full code"
            style={{ ...inputStyle, flex: "2 1 220px" }}
          />
          <input
            data-testid="markset-room-input"
            value={room}
            onChange={(e) => setRoom(e.currentTarget.value)}
            placeholder="Room"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <input
            data-testid="markset-day-input"
            value={day}
            onChange={(e) => setDay(e.currentTarget.value)}
            placeholder="Day"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <input
            data-testid="markset-period-input"
            value={period}
            onChange={(e) => setPeriod(e.currentTarget.value)}
            placeholder="Period"
            style={{ ...inputStyle, flex: "1 1 120px" }}
          />
          <input
            data-testid="markset-blocktitle-input"
            value={blockTitle}
            onChange={(e) => setBlockTitle(e.currentTarget.value)}
            placeholder="Block title"
            style={{ ...inputStyle, flex: "1 1 180px" }}
          />
          <select
            data-testid="markset-weightmethod-select"
            value={weightMethod}
            onChange={(e) => setWeightMethod(e.currentTarget.value)}
            style={{ ...inputStyle, flex: "1 1 180px" }}
          >
            <option value="0">Weighting: Entry</option>
            <option value="1">Weighting: Category</option>
            <option value="2">Weighting: Equal</option>
          </select>
          <select
            data-testid="markset-calcmethod-select"
            value={calcMethod}
            onChange={(e) => setCalcMethod(e.currentTarget.value)}
            style={{ ...inputStyle, flex: "1 1 180px" }}
          >
            <option value="0">Calc Method 0</option>
            <option value="1">Calc Method 1</option>
            <option value="2">Calc Method 2</option>
            <option value="3">Calc Method 3</option>
            <option value="4">Calc Method 4</option>
          </select>
          <button data-testid="markset-save-settings-btn" onClick={() => void saveMarkSetSettings()}>
            Save Settings
          </button>
        </div>
      </div>

      <div style={{ display: "flex", gap: 16, minHeight: 0 }}>
        <div
          style={{
            flex: "0 0 360px",
            border: "1px solid #ddd",
            borderRadius: 10,
            padding: 16,
            height: "calc(100vh - 220px)",
            overflow: "auto"
          }}
        >
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Categories</div>

          <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
            <input
              value={newCategoryName}
              onChange={(e) => setNewCategoryName(e.currentTarget.value)}
              placeholder="Name"
              style={{ ...inputStyle, flex: 1 }}
            />
            <input
              value={newCategoryWeight}
              onChange={(e) => setNewCategoryWeight(e.currentTarget.value)}
              placeholder="Weight"
              style={{ ...inputStyle, width: 90 }}
            />
            <button disabled={!canAddCategory} onClick={() => void addCategory()}>
              Add
            </button>
          </div>

          {categories.length === 0 ? (
            <div style={{ color: "#666" }}>(none yet)</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 8 }}>
              {categories.map((c) => (
                <div
                  key={c.id}
                  style={{
                    border: "1px solid #eee",
                    borderRadius: 10,
                    padding: 10
                  }}
                >
                  <div style={{ display: "flex", gap: 8 }}>
                    <input
                      value={c.name}
                      style={{ ...inputStyle, flex: 1 }}
                      onChange={(e) => updateCategoryLocal(c.id, { name: e.currentTarget.value })}
                      onBlur={() => void updateCategory(c.id, { name: c.name.trim() })}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                    <input
                      value={c.weight == null ? "" : String(c.weight)}
                      style={{ ...inputStyle, width: 90 }}
                      onChange={(e) =>
                        updateCategoryLocal(c.id, {
                          weight: parseNullableNumber(e.currentTarget.value)
                        })
                      }
                      onBlur={() =>
                        void updateCategory(c.id, {
                          weight: c.weight == null ? null : c.weight
                        })
                      }
                      onKeyDown={(e) => {
                        if (e.key === "Enter") (e.currentTarget as any).blur();
                      }}
                    />
                    <button
                      onClick={() => void deleteCategory(c.id)}
                      style={{ color: "#b00020" }}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        <div style={{ flex: 1, minWidth: 0 }}>
          <div
            style={{
              border: "1px solid #ddd",
              borderRadius: 10,
              padding: 16,
              marginBottom: 16
            }}
          >
            <div style={{ fontWeight: 700, marginBottom: 8 }}>Add Assessment</div>
            <div style={{ display: "flex", gap: 8, flexWrap: "wrap" }}>
              <input
                value={newTitle}
                onChange={(e) => setNewTitle(e.currentTarget.value)}
                placeholder="Title"
                style={{ ...inputStyle, flex: "2 1 240px" }}
              />
              <input
                value={newDate}
                onChange={(e) => setNewDate(e.currentTarget.value)}
                placeholder="Date (YYYY-MM-DD)"
                style={{ ...inputStyle, flex: "1 1 160px" }}
              />
              <input
                value={newCategoryName2}
                onChange={(e) => setNewCategoryName2(e.currentTarget.value)}
                placeholder="Category"
                style={{ ...inputStyle, flex: "1 1 140px" }}
              />
              <input
                value={newTerm}
                onChange={(e) => setNewTerm(e.currentTarget.value)}
                placeholder="Term"
                style={{ ...inputStyle, width: 90 }}
              />
              <input
                value={newWeight}
                onChange={(e) => setNewWeight(e.currentTarget.value)}
                placeholder="Weight"
                style={{ ...inputStyle, width: 90 }}
              />
              <input
                value={newOutOf}
                onChange={(e) => setNewOutOf(e.currentTarget.value)}
                placeholder="Out of"
                style={{ ...inputStyle, width: 90 }}
              />
              <button disabled={!canAddAssessment} onClick={() => void addAssessment()}>
                Add
              </button>
            </div>
          </div>

          <div
            data-testid="assessments-table-wrap"
            style={{ overflow: "auto", border: "1px solid #eee", borderRadius: 10 }}
          >
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr style={{ background: "#fafafa", borderBottom: "1px solid #eee" }}>
                  <th style={{ textAlign: "left", padding: 10, width: 60 }}>#</th>
                  <th style={{ textAlign: "left", padding: 10, width: 260 }}>Title</th>
                  <th style={{ textAlign: "left", padding: 10, width: 150 }}>Date</th>
                  <th style={{ textAlign: "left", padding: 10, width: 140 }}>Category</th>
                  <th style={{ textAlign: "left", padding: 10, width: 90 }}>Term</th>
                  <th style={{ textAlign: "left", padding: 10, width: 110 }}>Weight</th>
                  <th style={{ textAlign: "left", padding: 10, width: 110 }}>Out Of</th>
                  <th style={{ textAlign: "left", padding: 10, width: 90 }} title="From .TYP">
                    Type
                  </th>
                  <th style={{ textAlign: "left", padding: 10, width: 220 }}>Actions</th>
                </tr>
              </thead>
              <tbody>
                {assessments.map((a, i) => (
                  <tr
                    key={a.id}
                    data-testid={`assessment-row-${a.id}`}
                    style={{ borderBottom: "1px solid #f0f0f0" }}
                  >
                    <td style={{ padding: 10, color: "#444" }}>{i + 1}</td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-title-${a.id}`}
                        value={a.title}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, { title: e.currentTarget.value })
                        }
                        onBlur={() => void updateAssessment(a.id, { title: a.title.trim() })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-date-${a.id}`}
                        value={a.date ?? ""}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, { date: e.currentTarget.value || null })
                        }
                        onBlur={() =>
                          void updateAssessment(a.id, {
                            date: (a.date ?? "").trim() || null
                          })
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-category-${a.id}`}
                        value={a.categoryName ?? ""}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            categoryName: e.currentTarget.value || null
                          })
                        }
                        onBlur={() =>
                          void updateAssessment(a.id, {
                            categoryName: (a.categoryName ?? "").trim() || null
                          })
                        }
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-term-${a.id}`}
                        value={a.term == null ? "" : String(a.term)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            term: parseNullableInt(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { term: a.term })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-weight-${a.id}`}
                        value={a.weight == null ? "" : String(a.weight)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            weight: parseNullableNumber(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { weight: a.weight })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10 }}>
                      <input
                        data-testid={`assessment-outof-${a.id}`}
                        value={a.outOf == null ? "" : String(a.outOf)}
                        style={inputStyle}
                        onChange={(e) =>
                          updateAssessmentLocal(a.id, {
                            outOf: parseNullableNumber(e.currentTarget.value)
                          })
                        }
                        onBlur={() => void updateAssessment(a.id, { outOf: a.outOf })}
                        onKeyDown={(e) => {
                          if (e.key === "Enter") (e.currentTarget as any).blur();
                        }}
                      />
                    </td>
                    <td style={{ padding: 10, color: "#555" }}>
                      {a.legacyType == null ? "" : String(a.legacyType)}
                    </td>
                    <td style={{ padding: 10 }}>
                      <div style={{ display: "flex", gap: 8, alignItems: "center" }}>
                        <button
                          data-testid={`assessment-move-up-${a.id}`}
                          disabled={i === 0}
                          onClick={() => void moveAssessment(i, -1)}
                        >
                          Up
                        </button>
                        <button
                          data-testid={`assessment-move-down-${a.id}`}
                          disabled={i === assessments.length - 1}
                          onClick={() => void moveAssessment(i, 1)}
                        >
                          Down
                        </button>
                        <button
                          data-testid={`assessment-delete-${a.id}`}
                          onClick={() => void deleteAssessment(a.id)}
                          style={{ color: "#b00020" }}
                        >
                          Delete
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          <div style={{ marginTop: 12, fontSize: 12, color: "#666" }}>
            Reordering assessments changes column order in the Marks grid.
          </div>
        </div>
      </div>
        </>
      ) : (
        <MarkSetCommentsPanel
          selectedClassId={props.selectedClassId}
          selectedMarkSetId={props.selectedMarkSetId}
          onError={props.onError}
        />
      )}
    </div>
  );
}
