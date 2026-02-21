import React, { useState } from "react";
import {
  ExchangeApplyClassCsvResultSchema,
  ExchangeExportClassCsvResultSchema,
  ExchangePreviewClassCsvResultSchema,
  ExchangeImportClassCsvResultSchema,
  IntegrationsAdminTransferApplyPackageResultSchema,
  IntegrationsAdminTransferExportPackageResultSchema,
  IntegrationsAdminTransferPreviewPackageResultSchema,
  IntegrationsSisApplyImportResultSchema,
  IntegrationsSisExportMarksResultSchema,
  IntegrationsSisExportRosterResultSchema,
  IntegrationsSisPreviewImportResultSchema,
  MarkSetsListResultSchema,
  SetupGetResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

export function ExchangeScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [tab, setTab] = useState<"class" | "sis" | "admin">("class");
  const [exportPath, setExportPath] = useState("");
  const [importPath, setImportPath] = useState("");
  const [mode, setMode] = useState<"upsert" | "replace">("upsert");
  const [sisPath, setSisPath] = useState("");
  const [sisExportRosterPath, setSisExportRosterPath] = useState("");
  const [sisExportMarksPath, setSisExportMarksPath] = useState("");
  const [sisProfile, setSisProfile] = useState<"mb_exchange_v1" | "sis_roster_v1" | "sis_marks_v1">(
    "sis_roster_v1"
  );
  const [sisMatchMode, setSisMatchMode] = useState<"student_no_then_name" | "name_only" | "sort_order">(
    "student_no_then_name"
  );
  const [sisCollisionPolicy, setSisCollisionPolicy] = useState<
    "merge_existing" | "append_new" | "stop_on_collision"
  >("merge_existing");
  const [sisMode, setSisMode] = useState<"upsert_preserve" | "replace_snapshot">("upsert_preserve");
  const [sisAutoPreviewBeforeApply, setSisAutoPreviewBeforeApply] = useState(true);
  const [sisPreviewReady, setSisPreviewReady] = useState(false);
  const [markSets, setMarkSets] = useState<Array<{ id: string; code: string }>>([]);
  const [sisMarkSetId, setSisMarkSetId] = useState<string>("");
  const [adminPath, setAdminPath] = useState("");
  const [adminExportPath, setAdminExportPath] = useState("");
  const [adminCommentPolicy, setAdminCommentPolicy] = useState<
    "replace" | "append" | "fill_blank" | "source_if_longer"
  >("fill_blank");
  const [adminPreviewReady, setAdminPreviewReady] = useState(false);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string>("");
  const [preview, setPreview] = useState<{
    rowsTotal: number;
    rowsParsed: number;
    rowsMatched: number;
    rowsUnmatched: number;
    warningsCount: number;
  } | null>(null);
  const [sisPreview, setSisPreview] = useState<{
    rowsTotal: number;
    rowsParsed: number;
    matched: number;
    newCount: number;
    ambiguous: number;
    invalid: number;
  } | null>(null);
  const [adminPreview, setAdminPreview] = useState<{
    sourceRows: number;
    targetRows: number;
    matched: number;
    unmatchedSource: number;
    ambiguous: number;
    markSetCount: number;
  } | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    async function loadDefaults() {
      try {
        const setup = await requestParsed("setup.get", {}, SetupGetResultSchema);
        if (cancelled) return;
        setSisProfile(setup.integrations.defaultSisProfile);
        setSisMatchMode(setup.integrations.defaultMatchMode);
        setSisCollisionPolicy(setup.integrations.defaultCollisionPolicy);
        setSisAutoPreviewBeforeApply(setup.integrations.autoPreviewBeforeApply);
        setAdminCommentPolicy(setup.integrations.adminTransferDefaultPolicy);
      } catch {
        // best-effort; defaults already present locally
      }
    }
    void loadDefaults();
    return () => {
      cancelled = true;
    };
  }, []);

  React.useEffect(() => {
    let cancelled = false;
    async function loadMarkSets() {
      try {
        const res = await requestParsed(
          "marksets.list",
          { classId: props.selectedClassId, includeDeleted: false },
          MarkSetsListResultSchema
        );
        if (cancelled) return;
        const next = res.markSets.map((m) => ({ id: m.id, code: m.code }));
        setMarkSets(next);
        if (!next.some((m) => m.id === sisMarkSetId)) {
          setSisMarkSetId(next[0]?.id ?? "");
        }
      } catch {
        if (!cancelled) {
          setMarkSets([]);
          setSisMarkSetId("");
        }
      }
    }
    void loadMarkSets();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId, sisMarkSetId]);

  async function browseExportPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickSave({
        title: "Export Class CSV",
        defaultPath: "class-exchange.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setExportPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseImportPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickOpen({
        title: "Import Class CSV",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setImportPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseSisPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickOpen({
        title: "SIS Import CSV",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setSisPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseSisExportRosterPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickSave({
        title: "Export SIS Roster CSV",
        defaultPath: "sis-roster.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setSisExportRosterPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseSisExportMarksPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickSave({
        title: "Export SIS Marks CSV",
        defaultPath: "sis-marks.csv",
        filters: [{ name: "CSV", extensions: ["csv"] }]
      });
      if (picked) setSisExportMarksPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseAdminPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickOpen({
        title: "Open Admin Transfer Package",
        filters: [{ name: "ZIP", extensions: ["zip"] }]
      });
      if (picked) setAdminPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function browseAdminExportPath() {
    props.onError(null);
    try {
      const picked = await window.markbook.files.pickSave({
        title: "Export Admin Transfer Package",
        defaultPath: "admin-transfer.zip",
        filters: [{ name: "ZIP", extensions: ["zip"] }]
      });
      if (picked) setAdminExportPath(picked);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    }
  }

  async function exportCsv() {
    if (!exportPath.trim()) {
      props.onError("Enter an export path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.exportClassCsv",
        { classId: props.selectedClassId, outPath: exportPath.trim() },
        ExchangeExportClassCsvResultSchema
      );
      setStatus(`Exported ${res.rowsExported} rows to ${res.path}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function previewCsv() {
    if (!importPath.trim()) {
      props.onError("Enter an import path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.previewClassCsv",
        {
          classId: props.selectedClassId,
          inPath: importPath.trim(),
          mode
        },
        ExchangePreviewClassCsvResultSchema
      );
      setPreview({
        rowsTotal: res.rowsTotal,
        rowsParsed: res.rowsParsed,
        rowsMatched: res.rowsMatched,
        rowsUnmatched: res.rowsUnmatched,
        warningsCount: res.warningsCount
      });
      setStatus(`Preview: matched ${res.rowsMatched}/${res.rowsParsed} parsed rows.`);
    } catch (e: any) {
      setPreview(null);
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function previewSis() {
    if (!sisPath.trim()) {
      props.onError("Enter an SIS import path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.sis.previewImport",
        {
          classId: props.selectedClassId,
          inPath: sisPath.trim(),
          profile: sisProfile,
          matchMode: sisMatchMode,
          mode: sisMode
        },
        IntegrationsSisPreviewImportResultSchema
      );
      setSisPreview({
        rowsTotal: res.rowsTotal,
        rowsParsed: res.rowsParsed,
        matched: res.matched,
        newCount: res.new,
        ambiguous: res.ambiguous,
        invalid: res.invalid
      });
      setSisPreviewReady(true);
      setStatus(
        `SIS preview: matched ${res.matched}, new ${res.new}, ambiguous ${res.ambiguous}, invalid ${res.invalid}.`
      );
    } catch (e: any) {
      setSisPreviewReady(false);
      setSisPreview(null);
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function applySis() {
    if (!sisPath.trim()) {
      props.onError("Enter an SIS import path first.");
      return;
    }
    if (sisAutoPreviewBeforeApply && !sisPreviewReady) {
      props.onError("Run SIS preview before applying.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.sis.applyImport",
        {
          classId: props.selectedClassId,
          inPath: sisPath.trim(),
          profile: sisProfile,
          matchMode: sisMatchMode,
          mode: sisMode,
          collisionPolicy: sisCollisionPolicy
        },
        IntegrationsSisApplyImportResultSchema
      );
      setStatus(
        `SIS apply complete: created ${res.created}, updated ${res.updated}, ambiguous skipped ${res.ambiguousSkipped}.`
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportSisRoster() {
    if (!sisExportRosterPath.trim()) {
      props.onError("Enter SIS roster export path.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.sis.exportRoster",
        {
          classId: props.selectedClassId,
          outPath: sisExportRosterPath.trim(),
          profile: sisProfile
        },
        IntegrationsSisExportRosterResultSchema
      );
      setStatus(`Exported SIS roster: ${res.rowsExported} rows -> ${res.path}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportSisMarks() {
    if (!sisExportMarksPath.trim()) {
      props.onError("Enter SIS marks export path.");
      return;
    }
    if (!sisMarkSetId) {
      props.onError("Select a mark set for SIS marks export.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.sis.exportMarks",
        {
          classId: props.selectedClassId,
          markSetId: sisMarkSetId,
          outPath: sisExportMarksPath.trim(),
          profile: "sis_marks_v1",
          includeStateColumns: true
        },
        IntegrationsSisExportMarksResultSchema
      );
      setStatus(
        `Exported SIS marks: ${res.rowsExported} rows across ${res.assessmentsExported} assessments -> ${res.path}`
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function previewAdminPackage() {
    if (!adminPath.trim()) {
      props.onError("Enter admin transfer package path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.adminTransfer.previewPackage",
        {
          targetClassId: props.selectedClassId,
          inPath: adminPath.trim(),
          matchMode: sisMatchMode
        },
        IntegrationsAdminTransferPreviewPackageResultSchema
      );
      setAdminPreview({
        sourceRows: res.studentAlignment.sourceRows,
        targetRows: res.studentAlignment.targetRows,
        matched: res.studentAlignment.matched,
        unmatchedSource: res.studentAlignment.unmatchedSource,
        ambiguous: res.studentAlignment.ambiguous,
        markSetCount: res.markSetCount
      });
      setAdminPreviewReady(true);
      setStatus(
        `Admin preview: mark sets ${res.markSetCount}, matched ${res.studentAlignment.matched}, unmatched ${res.studentAlignment.unmatchedSource}, ambiguous ${res.studentAlignment.ambiguous}.`
      );
    } catch (e: any) {
      setAdminPreviewReady(false);
      setAdminPreview(null);
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function applyAdminPackage() {
    if (!adminPath.trim()) {
      props.onError("Enter admin transfer package path first.");
      return;
    }
    if (sisAutoPreviewBeforeApply && !adminPreviewReady) {
      props.onError("Run admin transfer preview before applying.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.adminTransfer.applyPackage",
        {
          targetClassId: props.selectedClassId,
          inPath: adminPath.trim(),
          matchMode: sisMatchMode,
          collisionPolicy: sisCollisionPolicy,
          commentPolicy: adminCommentPolicy
        },
        IntegrationsAdminTransferApplyPackageResultSchema
      );
      setStatus(
        `Admin apply complete: assessments created ${res.assessments.created}, merged ${res.assessments.merged}, scores upserted ${res.scores.upserted}.`
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function exportAdminPackage() {
    if (!adminExportPath.trim()) {
      props.onError("Enter admin transfer export path.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "integrations.adminTransfer.exportPackage",
        {
          classId: props.selectedClassId,
          outPath: adminExportPath.trim(),
          includeComments: true,
          includeLearningSkills: true
        },
        IntegrationsAdminTransferExportPackageResultSchema
      );
      setStatus(`Exported admin package (${res.format}) with ${res.entriesWritten} entries -> ${res.path}`);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setBusy(false);
    }
  }

  async function importCsv() {
    if (!importPath.trim()) {
      props.onError("Enter an import path first.");
      return;
    }
    setBusy(true);
    setStatus("");
    props.onError(null);
    try {
      const res = await requestParsed(
        "exchange.applyClassCsv",
        {
          classId: props.selectedClassId,
          inPath: importPath.trim(),
          mode
        },
        ExchangeApplyClassCsvResultSchema
      );
      setStatus(
        `Applied ${res.updated} score rows (${mode}); skipped ${res.skipped}, warnings ${res.warningsCount}.`
      );
      setPreview({
        rowsTotal: res.rowsTotal,
        rowsParsed: res.rowsParsed,
        rowsMatched: res.updated,
        rowsUnmatched: res.skipped,
        warningsCount: res.warningsCount
      });
    } catch (e: any) {
      // Fall back to legacy import endpoint for older sidecars.
      try {
        const fallback = await requestParsed(
          "exchange.importClassCsv",
          {
            classId: props.selectedClassId,
            inPath: importPath.trim(),
            mode
          },
          ExchangeImportClassCsvResultSchema
        );
        setStatus(`Imported ${fallback.updated} score rows (${mode}).`);
      } catch (fallbackErr: any) {
        props.onError(fallbackErr?.message ?? String(fallbackErr));
      }
    } finally {
      setBusy(false);
    }
  }

  return (
    <div data-testid="exchange-screen" style={{ padding: 24, maxWidth: 920 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>Exchange</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button
          data-testid="exchange-tab-class"
          onClick={() => setTab("class")}
          disabled={busy || tab === "class"}
        >
          Class Exchange
        </button>
        <button
          data-testid="integrations-sis-tab"
          onClick={() => setTab("sis")}
          disabled={busy || tab === "sis"}
        >
          SIS
        </button>
        <button
          data-testid="integrations-admin-tab"
          onClick={() => setTab("admin")}
          disabled={busy || tab === "admin"}
        >
          Admin Transfer
        </button>
      </div>

      {tab === "class" ? (
      <>
      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14, marginBottom: 12 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Export Class CSV</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
          <input
            data-testid="exchange-export-path-input"
            value={exportPath}
            onChange={(e) => setExportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/class-export.csv"
            style={{ flex: "1 1 560px", padding: "6px 8px" }}
          />
          <button
            data-testid="exchange-export-browse-btn"
            disabled={busy}
            onClick={() => void browseExportPath()}
          >
            Browse
          </button>
          <button data-testid="exchange-export-btn" disabled={busy} onClick={() => void exportCsv()}>
            {busy ? "Working..." : "Export"}
          </button>
        </div>
      </div>

      <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14 }}>
        <div style={{ fontWeight: 700, marginBottom: 6 }}>Import Class CSV</div>
        <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
          <input
            data-testid="exchange-import-path-input"
            value={importPath}
            onChange={(e) => setImportPath(e.currentTarget.value)}
            placeholder="/absolute/path/to/class-export.csv"
            style={{ flex: "1 1 560px", padding: "6px 8px" }}
          />
          <button
            data-testid="exchange-import-browse-btn"
            disabled={busy}
            onClick={() => void browseImportPath()}
          >
            Browse
          </button>
          <label>
            Mode{" "}
            <select
              data-testid="exchange-import-mode-select"
              value={mode}
              onChange={(e) => setMode((e.currentTarget.value as any) || "upsert")}
            >
              <option value="upsert">Upsert</option>
              <option value="replace">Replace</option>
            </select>
          </label>
          <button data-testid="exchange-preview-btn" disabled={busy} onClick={() => void previewCsv()}>
            {busy ? "Working..." : "Preview"}
          </button>
          <button data-testid="exchange-import-btn" disabled={busy} onClick={() => void importCsv()}>
            {busy ? "Working..." : "Import"}
          </button>
        </div>
      </div>
      </>
      ) : null}

      {tab === "sis" ? (
        <>
          <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14, marginBottom: 12 }}>
            <div style={{ fontWeight: 700, marginBottom: 6 }}>SIS Import</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              <label>
                Profile
                <select
                  value={sisProfile}
                  onChange={(e) => setSisProfile(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="sis_roster_v1">sis_roster_v1</option>
                  <option value="sis_marks_v1">sis_marks_v1</option>
                  <option value="mb_exchange_v1">mb_exchange_v1</option>
                </select>
              </label>
              <label>
                Match mode
                <select
                  value={sisMatchMode}
                  onChange={(e) => setSisMatchMode(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="student_no_then_name">student_no_then_name</option>
                  <option value="name_only">name_only</option>
                  <option value="sort_order">sort_order</option>
                </select>
              </label>
              <label>
                Import mode
                <select
                  value={sisMode}
                  onChange={(e) => setSisMode(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="upsert_preserve">upsert_preserve</option>
                  <option value="replace_snapshot">replace_snapshot</option>
                </select>
              </label>
              <label>
                Collision policy
                <select
                  value={sisCollisionPolicy}
                  onChange={(e) => setSisCollisionPolicy(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="merge_existing">merge_existing</option>
                  <option value="append_new">append_new</option>
                  <option value="stop_on_collision">stop_on_collision</option>
                </select>
              </label>
            </div>
            <label style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 8 }}>
              <input
                type="checkbox"
                checked={sisAutoPreviewBeforeApply}
                onChange={(e) => setSisAutoPreviewBeforeApply(e.currentTarget.checked)}
              />
              Require preview before apply
            </label>
            <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <input
                data-testid="integrations-sis-path-input"
                value={sisPath}
                onChange={(e) => setSisPath(e.currentTarget.value)}
                placeholder="/absolute/path/to/sis-import.csv"
                style={{ flex: "1 1 560px", padding: "6px 8px" }}
              />
              <button disabled={busy} onClick={() => void browseSisPath()}>
                Browse
              </button>
              <button
                data-testid="integrations-sis-preview-btn"
                disabled={busy}
                onClick={() => void previewSis()}
              >
                {busy ? "Working..." : "Preview"}
              </button>
              <button
                data-testid="integrations-sis-apply-btn"
                disabled={busy}
                onClick={() => void applySis()}
              >
                {busy ? "Working..." : "Apply"}
              </button>
            </div>
            {sisPreview ? (
              <div style={{ marginTop: 8, color: "#444", fontSize: 13 }}>
                Parsed {sisPreview.rowsParsed}/{sisPreview.rowsTotal}, matched {sisPreview.matched}, new{" "}
                {sisPreview.newCount}, ambiguous {sisPreview.ambiguous}, invalid {sisPreview.invalid}
              </div>
            ) : null}
          </div>

          <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14 }}>
            <div style={{ fontWeight: 700, marginBottom: 6 }}>SIS Exports</div>
            <div style={{ marginBottom: 8 }}>
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <input
                  value={sisExportRosterPath}
                  onChange={(e) => setSisExportRosterPath(e.currentTarget.value)}
                  placeholder="/absolute/path/to/sis-roster.csv"
                  style={{ flex: "1 1 560px", padding: "6px 8px" }}
                />
                <button disabled={busy} onClick={() => void browseSisExportRosterPath()}>
                  Browse
                </button>
                <button
                  data-testid="integrations-sis-export-roster-btn"
                  disabled={busy}
                  onClick={() => void exportSisRoster()}
                >
                  {busy ? "Working..." : "Export Roster"}
                </button>
              </div>
            </div>
            <div style={{ display: "grid", gridTemplateColumns: "160px 1fr auto auto", gap: 8 }}>
              <select value={sisMarkSetId} onChange={(e) => setSisMarkSetId(e.currentTarget.value)}>
                {markSets.map((m) => (
                  <option key={m.id} value={m.id}>
                    {m.code}
                  </option>
                ))}
              </select>
              <input
                value={sisExportMarksPath}
                onChange={(e) => setSisExportMarksPath(e.currentTarget.value)}
                placeholder="/absolute/path/to/sis-marks.csv"
                style={{ padding: "6px 8px" }}
              />
              <button disabled={busy} onClick={() => void browseSisExportMarksPath()}>
                Browse
              </button>
              <button
                data-testid="integrations-sis-export-marks-btn"
                disabled={busy}
                onClick={() => void exportSisMarks()}
              >
                {busy ? "Working..." : "Export Marks"}
              </button>
            </div>
          </div>
        </>
      ) : null}

      {tab === "admin" ? (
        <>
          <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14, marginBottom: 12 }}>
            <div style={{ fontWeight: 700, marginBottom: 6 }}>Admin Transfer Import</div>
            <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
              <label>
                Match mode
                <select
                  value={sisMatchMode}
                  onChange={(e) => setSisMatchMode(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="student_no_then_name">student_no_then_name</option>
                  <option value="name_only">name_only</option>
                  <option value="sort_order">sort_order</option>
                </select>
              </label>
              <label>
                Collision policy
                <select
                  value={sisCollisionPolicy}
                  onChange={(e) => setSisCollisionPolicy(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="merge_existing">merge_existing</option>
                  <option value="append_new">append_new</option>
                  <option value="stop_on_collision">stop_on_collision</option>
                </select>
              </label>
              <label>
                Comments policy
                <select
                  data-testid="integrations-admin-comment-policy"
                  value={adminCommentPolicy}
                  onChange={(e) => setAdminCommentPolicy(e.currentTarget.value as any)}
                  style={{ display: "block", marginTop: 4, width: "100%" }}
                >
                  <option value="fill_blank">fill_blank</option>
                  <option value="replace">replace</option>
                  <option value="append">append</option>
                  <option value="source_if_longer">source_if_longer</option>
                </select>
              </label>
            </div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <input
                value={adminPath}
                onChange={(e) => setAdminPath(e.currentTarget.value)}
                placeholder="/absolute/path/to/admin-transfer.zip"
                style={{ flex: "1 1 560px", padding: "6px 8px" }}
              />
              <button disabled={busy} onClick={() => void browseAdminPath()}>
                Browse
              </button>
              <button
                data-testid="integrations-admin-preview-btn"
                disabled={busy}
                onClick={() => void previewAdminPackage()}
              >
                {busy ? "Working..." : "Preview"}
              </button>
              <button
                data-testid="integrations-admin-apply-btn"
                disabled={busy}
                onClick={() => void applyAdminPackage()}
              >
                {busy ? "Working..." : "Apply"}
              </button>
            </div>
            {adminPreview ? (
              <div style={{ marginTop: 8, color: "#444", fontSize: 13 }}>
                Source rows {adminPreview.sourceRows}, target rows {adminPreview.targetRows}, matched{" "}
                {adminPreview.matched}, unmatched {adminPreview.unmatchedSource}, ambiguous{" "}
                {adminPreview.ambiguous}, mark sets {adminPreview.markSetCount}
              </div>
            ) : null}
          </div>

          <div style={{ border: "1px solid #ddd", borderRadius: 10, padding: 14 }}>
            <div style={{ fontWeight: 700, marginBottom: 6 }}>Admin Transfer Export</div>
            <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
              <input
                value={adminExportPath}
                onChange={(e) => setAdminExportPath(e.currentTarget.value)}
                placeholder="/absolute/path/to/admin-transfer.zip"
                style={{ flex: "1 1 560px", padding: "6px 8px" }}
              />
              <button disabled={busy} onClick={() => void browseAdminExportPath()}>
                Browse
              </button>
              <button
                data-testid="integrations-admin-export-btn"
                disabled={busy}
                onClick={() => void exportAdminPackage()}
              >
                {busy ? "Working..." : "Export Package"}
              </button>
            </div>
          </div>
        </>
      ) : null}

      {preview ? (
        <div
          data-testid="exchange-preview-summary"
          style={{ marginTop: 8, color: "#444", fontSize: 13 }}
        >
          Parsed {preview.rowsParsed}/{preview.rowsTotal}, matched {preview.rowsMatched}, unmatched{" "}
          {preview.rowsUnmatched}, warnings {preview.warningsCount}
        </div>
      ) : null}
      {status ? <div style={{ marginTop: 8, color: "#1a5" }}>{status}</div> : null}
      <div style={{ marginTop: 10, color: "#666", fontSize: 12 }}>
        Exchange and integrations run through typed sidecar IPC endpoints. Preview-first workflows
        are recommended before apply.
      </div>
    </div>
  );
}
