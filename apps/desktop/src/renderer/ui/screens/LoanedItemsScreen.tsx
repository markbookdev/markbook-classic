import React, { useEffect, useMemo, useState } from "react";
import {
  LoanedListResultSchema,
  LoanedUpdateResultSchema,
  MarkSetsListResultSchema,
  StudentsListResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type LoanedItem = {
  id: string;
  studentId: string;
  displayName: string;
  markSetId: string | null;
  itemName: string;
  quantity: number | null;
  notes: string | null;
  rawLine: string;
};

type StudentRow = {
  id: string;
  displayName: string;
};

type MarkSetRow = {
  id: string;
  code: string;
  description: string;
};

export function LoanedItemsScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [items, setItems] = useState<LoanedItem[]>([]);
  const [students, setStudents] = useState<StudentRow[]>([]);
  const [markSets, setMarkSets] = useState<MarkSetRow[]>([]);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState("");

  const [filterMarkSetId, setFilterMarkSetId] = useState<string>("ALL");

  const [editingItemId, setEditingItemId] = useState<string | null>(null);
  const [studentId, setStudentId] = useState("");
  const [markSetId, setMarkSetId] = useState<string>("");
  const [itemName, setItemName] = useState("");
  const [quantity, setQuantity] = useState("");
  const [notes, setNotes] = useState("");

  const sortedItems = useMemo(
    () => [...items].sort((a, b) => a.displayName.localeCompare(b.displayName) || a.itemName.localeCompare(b.itemName)),
    [items]
  );

  function resetForm() {
    setEditingItemId(null);
    setStudentId(students[0]?.id ?? "");
    setMarkSetId("");
    setItemName("");
    setQuantity("");
    setNotes("");
  }

  function loadIntoForm(item: LoanedItem) {
    setEditingItemId(item.id);
    setStudentId(item.studentId);
    setMarkSetId(item.markSetId ?? "");
    setItemName(item.itemName);
    setQuantity(item.quantity == null ? "" : String(item.quantity));
    setNotes(item.notes ?? "");
  }

  async function load() {
    setBusy(true);
    props.onError(null);
    setStatus("");
    try {
      const [studentsRes, markSetsRes, itemsRes] = await Promise.all([
        requestParsed("students.list", { classId: props.selectedClassId }, StudentsListResultSchema),
        requestParsed("marksets.list", { classId: props.selectedClassId }, MarkSetsListResultSchema),
        requestParsed(
          "loaned.list",
          {
            classId: props.selectedClassId,
            markSetId: filterMarkSetId === "ALL" ? undefined : filterMarkSetId
          },
          LoanedListResultSchema
        )
      ]);
      setStudents(studentsRes.students.map((s) => ({ id: s.id, displayName: s.displayName })));
      setMarkSets(
        markSetsRes.markSets.map((m) => ({
          id: m.id,
          code: m.code,
          description: m.description
        }))
      );
      setItems(itemsRes.items as LoanedItem[]);

      if (!editingItemId) {
        setStudentId((prev) => prev || studentsRes.students[0]?.id || "");
      } else {
        const existing = itemsRes.items.find((x) => x.id === editingItemId);
        if (existing) {
          loadIntoForm(existing as LoanedItem);
        } else {
          resetForm();
        }
      }
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, filterMarkSetId]);

  async function saveItem() {
    if (!studentId) {
      props.onError("Select a student before saving.");
      return;
    }
    if (!itemName.trim()) {
      props.onError("Item name is required.");
      return;
    }

    let parsedQuantity: number | null = null;
    if (quantity.trim()) {
      const n = Number(quantity.trim());
      if (!Number.isFinite(n)) {
        props.onError("Quantity must be numeric.");
        return;
      }
      parsedQuantity = n;
    }

    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "loaned.update",
        {
          classId: props.selectedClassId,
          itemId: editingItemId ?? undefined,
          studentId,
          markSetId: markSetId || null,
          itemName: itemName.trim(),
          quantity: parsedQuantity,
          notes: notes.trim() ? notes.trim() : null,
          rawLine: ""
        },
        LoanedUpdateResultSchema
      );
      setEditingItemId(res.itemId);
      setStatus("Saved loaned item.");
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div data-testid="loaned-screen" style={{ padding: 24 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 10 }}>Loaned Items</div>

      <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 12, flexWrap: "wrap" }}>
        <label>
          Mark Set{" "}
          <select
            data-testid="loaned-filter-markset-select"
            value={filterMarkSetId}
            onChange={(e) => setFilterMarkSetId(e.currentTarget.value)}
            disabled={busy}
          >
            <option value="ALL">All</option>
            {markSets.map((m) => (
              <option key={m.id} value={m.id}>
                {m.code}: {m.description}
              </option>
            ))}
          </select>
        </label>
        <button data-testid="loaned-reload-btn" onClick={() => void load()} disabled={busy}>
          Reload
        </button>
        <button
          data-testid="loaned-new-btn"
          onClick={() => resetForm()}
          disabled={busy}
        >
          New Item
        </button>
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "minmax(320px, 1fr) minmax(360px, 1fr)", gap: 12 }}>
        <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 10, maxHeight: 520, overflow: "auto" }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Existing Items</div>
          {sortedItems.length === 0 ? (
            <div style={{ color: "#666" }}>(none)</div>
          ) : (
            <table style={{ width: "100%", borderCollapse: "collapse", fontSize: 13 }}>
              <thead>
                <tr>
                  <th style={{ textAlign: "left" }}>Student</th>
                  <th style={{ textAlign: "left" }}>Item</th>
                  <th style={{ textAlign: "left" }}>Qty</th>
                  <th style={{ textAlign: "left" }}>Edit</th>
                </tr>
              </thead>
              <tbody>
                {sortedItems.map((item) => (
                  <tr key={item.id} data-testid={`loaned-item-row-${item.id}`}>
                    <td>{item.displayName}</td>
                    <td>{item.itemName}</td>
                    <td>{item.quantity ?? ""}</td>
                    <td>
                      <button
                        data-testid={`loaned-edit-${item.id}`}
                        onClick={() => loadIntoForm(item)}
                        disabled={busy}
                      >
                        Edit
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>

        <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>
            {editingItemId ? "Edit Loaned Item" : "New Loaned Item"}
          </div>
          <div style={{ display: "grid", gap: 8 }}>
            <label>
              Student
              <br />
              <select
                data-testid="loaned-student-select"
                value={studentId}
                onChange={(e) => setStudentId(e.currentTarget.value)}
                disabled={busy}
                style={{ width: "100%" }}
              >
                <option value="">(select)</option>
                {students.map((s) => (
                  <option key={s.id} value={s.id}>
                    {s.displayName}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Mark Set (optional)
              <br />
              <select
                data-testid="loaned-markset-select"
                value={markSetId}
                onChange={(e) => setMarkSetId(e.currentTarget.value)}
                disabled={busy}
                style={{ width: "100%" }}
              >
                <option value="">(none)</option>
                {markSets.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.code}: {m.description}
                  </option>
                ))}
              </select>
            </label>
            <label>
              Item Name
              <br />
              <input
                data-testid="loaned-item-name-input"
                value={itemName}
                onChange={(e) => setItemName(e.currentTarget.value)}
                disabled={busy}
                style={{ width: "100%" }}
              />
            </label>
            <label>
              Quantity
              <br />
              <input
                data-testid="loaned-quantity-input"
                value={quantity}
                onChange={(e) => setQuantity(e.currentTarget.value)}
                disabled={busy}
                style={{ width: "100%" }}
              />
            </label>
            <label>
              Notes
              <br />
              <textarea
                data-testid="loaned-notes-input"
                value={notes}
                onChange={(e) => setNotes(e.currentTarget.value)}
                disabled={busy}
                rows={4}
                style={{ width: "100%" }}
              />
            </label>
            <div style={{ display: "flex", gap: 8 }}>
              <button
                data-testid="loaned-save-btn"
                onClick={() => void saveItem()}
                disabled={busy}
              >
                {busy ? "Working..." : "Save"}
              </button>
              <button
                data-testid="loaned-reset-btn"
                onClick={() => resetForm()}
                disabled={busy}
              >
                Reset
              </button>
            </div>
          </div>
        </div>
      </div>

      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
    </div>
  );
}
