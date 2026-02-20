import React, { useEffect, useMemo, useState } from "react";
import {
  ClassImportLegacyResultSchema,
  ClassesCreateResultSchema,
  ClassesDeleteResultSchema,
  ClassesLegacyPreviewResultSchema,
  ClassesListResultSchema,
  ClassesUpdateFromLegacyResultSchema,
  MarkSetsListResultSchema
} from "@markbook/schema";
import type { HealthState, Prefs, SidecarMeta } from "../state/workspace";
import { requestParsed } from "../state/workspace";
import { DashboardScreen } from "../screens/DashboardScreen";
import { MarksScreen } from "../screens/MarksScreen";
import { ReportsScreen } from "../screens/ReportsScreen";
import { StudentsScreen } from "../screens/StudentsScreen";
import { MarkSetSetupScreen } from "../screens/MarkSetSetupScreen";
import { AttendanceScreen } from "../screens/AttendanceScreen";
import { NotesScreen } from "../screens/NotesScreen";
import { SeatingPlanScreen } from "../screens/SeatingPlanScreen";
import { LearningSkillsScreen } from "../screens/LearningSkillsScreen";
import { BackupScreen } from "../screens/BackupScreen";
import { ExchangeScreen } from "../screens/ExchangeScreen";
import { LoanedItemsScreen } from "../screens/LoanedItemsScreen";
import { DeviceMappingsScreen } from "../screens/DeviceMappingsScreen";
import { CalcSettingsScreen } from "../screens/CalcSettingsScreen";
import { ClassWizardScreen } from "../screens/ClassWizardScreen";
import { ClassAnalyticsScreen } from "../screens/ClassAnalyticsScreen";
import { StudentAnalyticsScreen } from "../screens/StudentAnalyticsScreen";
import { CombinedAnalyticsScreen } from "../screens/CombinedAnalyticsScreen";

type Screen =
  | "dashboard"
  | "marks"
  | "class_wizard"
  | "reports"
  | "class_analytics"
  | "student_analytics"
  | "combined_analytics"
  | "students"
  | "markset_setup"
  | "attendance"
  | "notes"
  | "seating_plan"
  | "learning_skills"
  | "loaned_items"
  | "device_mappings"
  | "calc_settings"
  | "backup"
  | "exchange";

type ClassRow = {
  id: string;
  name: string;
  studentCount?: number;
  markSetCount?: number;
};

type MarkSetRow = {
  id: string;
  code: string;
  description: string;
  sortOrder: number;
};

export function AppShell() {
  const [health, setHealth] = useState<HealthState>({
    version: "0.0.0",
    sidecar: false,
    workspacePath: null
  });
  const [sidecarMeta, setSidecarMeta] = useState<SidecarMeta | null>(null);
  const [prefs, setPrefs] = useState<Prefs | null>(null);

  const [screen, setScreen] = useState<Screen>("dashboard");
  const [classWizardMode, setClassWizardMode] = useState<"create" | "edit">("create");
  const [sidecarError, setSidecarError] = useState<string | null>(null);
  const [lastGridEvent, setLastGridEvent] = useState<string | null>(null);

  const [classes, setClasses] = useState<ClassRow[]>([]);
  const [selectedClassId, setSelectedClassId] = useState<string | null>(null);
  const [legacyPreviewByClass, setLegacyPreviewByClass] = useState<{
    classId: string;
    data: any;
  } | null>(null);
  const [reportsPrefill, setReportsPrefill] = useState<{
    filters: { term: number | null; categoryName: string | null; typesMask: number | null };
    studentScope: "all" | "active" | "valid";
    studentId?: string | null;
    markSetIds?: string[] | null;
    nonce: number;
  } | null>(null);

  const [markSets, setMarkSets] = useState<MarkSetRow[]>([]);
  const [selectedMarkSetId, setSelectedMarkSetId] = useState<string | null>(null);

  async function refreshPrefs() {
    if (window.markbook?.prefs?.get) {
      setPrefs(await window.markbook.prefs.get());
    } else {
      setPrefs(null);
    }
  }

  async function refresh() {
    setSidecarError(null);
    try {
      const h = await window.markbook.request("health", {});
      setHealth({
        version: String(h?.version ?? "0.0.0"),
        sidecar: true,
        workspacePath: h?.workspacePath ?? null
      });
      if (typeof (window.markbook as any).getSidecarMeta === "function") {
        setSidecarMeta(await window.markbook.getSidecarMeta());
      } else {
        setSidecarMeta(null);
      }
      const cls = await requestParsed("classes.list", {}, ClassesListResultSchema);
      setClasses(cls.classes as any);
      setSelectedClassId((cur) => {
        if (!cur) return cur;
        return cls.classes.some((c) => c.id === cur) ? cur : null;
      });
    } catch (e: any) {
      setHealth((x) => ({ ...x, sidecar: false }));
      setSidecarError(e?.message ?? String(e));
    }
  }

  useEffect(() => {
    let cancelled = false;
    async function boot() {
      let p: Prefs | null = null;
      if (window.markbook?.prefs?.get) {
        try {
          p = await window.markbook.prefs.get();
          setPrefs(p);
        } catch {
          p = null;
          setPrefs(null);
        }
      }

      // Best-effort auto-open last workspace.
      const last = p?.lastWorkspace ?? null;
      if (last) {
        try {
          await window.markbook.request("workspace.select", { path: last });
        } catch {
          // ignore; user can re-select.
        }
      }
      if (!cancelled) await refresh();
    }
    boot();
    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    let cancelled = false;
    async function run() {
      if (!selectedClassId) {
        setMarkSets([]);
        setSelectedMarkSetId(null);
        return;
      }
      setSidecarError(null);
      try {
        const ms = await requestParsed(
          "marksets.list",
          { classId: selectedClassId },
          MarkSetsListResultSchema
        );
        if (cancelled) return;
        setMarkSets(ms.markSets);
        setSelectedMarkSetId((cur) => {
          if (cur && ms.markSets.some((m) => m.id === cur)) return cur;
          return ms.markSets[0]?.id ?? null;
        });
      } catch (e: any) {
        if (cancelled) return;
        setSidecarError(e?.message ?? String(e));
        setMarkSets([]);
        setSelectedMarkSetId(null);
      }
    }
    run();
    return () => {
      cancelled = true;
    };
  }, [selectedClassId]);

  const selectedClass = useMemo(
    () => classes.find((c) => c.id === selectedClassId) ?? null,
    [classes, selectedClassId]
  );

  async function openWorkspacePath(wsPath: string) {
    if (!wsPath) return;
    setSidecarError(null);
    try {
      await window.markbook.request("workspace.select", { path: wsPath });
      if (window.markbook?.prefs?.setLastWorkspace) {
        await window.markbook.prefs.setLastWorkspace(wsPath);
        await refreshPrefs();
      }
      await refresh();
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  async function selectWorkspaceDialog() {
    setSidecarError(null);
    const wsPath = await window.markbook.selectWorkspace();
    if (!wsPath) return;
    await refreshPrefs();
    await refresh();
  }

  async function importLegacyClassFolder() {
    setSidecarError(null);
    try {
      let wsPath = health.workspacePath;
      if (!wsPath) {
        wsPath = await window.markbook.selectWorkspace();
        if (!wsPath) return;
        await refreshPrefs();
        await refresh();
      }
      const folder = await window.markbook.selectLegacyClassFolder();
      if (!folder) return;
      const res = await requestParsed(
        "class.importLegacy",
        { legacyClassFolderPath: folder },
        ClassImportLegacyResultSchema
      );
      setSelectedClassId(res.classId);
      setScreen("marks");
      await refresh();
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  async function createClass(name: string) {
    setSidecarError(null);
    try {
      const res = await requestParsed(
        "classes.create",
        { name },
        ClassesCreateResultSchema
      );
      setSelectedClassId(res.classId);
      setScreen("students");
      await refresh();
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  async function previewLegacyUpdate(classId: string) {
    setSidecarError(null);
    try {
      const folder = await window.markbook.selectLegacyClassFolder();
      if (!folder) return;
      const preview = await requestParsed(
        "classes.legacyPreview",
        { classId, legacyClassFolderPath: folder },
        ClassesLegacyPreviewResultSchema
      );
      setLegacyPreviewByClass({
        classId,
        data: {
          ...preview,
          legacyClassFolderPath: folder
        }
      });
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  async function updateFromLegacy(classId: string) {
    setSidecarError(null);
    try {
      const existingFolder =
        legacyPreviewByClass?.classId === classId
          ? legacyPreviewByClass?.data?.legacyClassFolderPath
          : null;
      const folder =
        typeof existingFolder === "string" && existingFolder.trim().length > 0
          ? existingFolder
          : await window.markbook.selectLegacyClassFolder();
      if (!folder) return;

      const res = await requestParsed(
        "classes.updateFromLegacy",
        {
          classId,
          legacyClassFolderPath: folder,
          mode: "upsert_preserve",
          collisionPolicy: "merge_existing",
          preserveLocalValidity: true
        },
        ClassesUpdateFromLegacyResultSchema
      );
      setLegacyPreviewByClass({
        classId,
        data: {
          ...(legacyPreviewByClass?.classId === classId ? legacyPreviewByClass.data : {}),
          legacyClassFolderPath: folder,
          lastUpdateResult: res
        }
      });
      setSelectedClassId(classId);
      await refresh();
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  async function deleteClass(classId: string) {
    if (!classId) return;
    const ok = confirm("Delete this class and all related data? This cannot be undone.");
    if (!ok) return;
    setSidecarError(null);
    try {
      await requestParsed("classes.delete", { classId }, ClassesDeleteResultSchema);
      setSelectedClassId((cur) => (cur === classId ? null : cur));
      if (selectedMarkSetId) setSelectedMarkSetId(null);
      setScreen("dashboard");
      await refresh();
    } catch (e: any) {
      setSidecarError(e?.message ?? String(e));
    }
  }

  return (
    <div
      data-testid="app-shell"
      style={{ height: "100vh", display: "flex", flexDirection: "column" }}
    >
      <div
        style={{
          padding: 12,
          display: "flex",
          alignItems: "center",
          gap: 12,
          borderBottom: "1px solid #ddd"
        }}
      >
        <div style={{ fontWeight: 800 }}>MarkBook Classic</div>
        <div style={{ color: "#555" }}>sidecar: {health.sidecar ? "ok" : "down"}</div>
        <div style={{ color: "#555" }}>version: {health.version}</div>
        <div style={{ color: "#555" }} title={sidecarMeta?.path ?? ""}>
          sidecar bin:{" "}
          {sidecarMeta?.path
            ? sidecarMeta.path.split("/").slice(-3).join("/")
            : "(unknown)"}
        </div>
        <div style={{ color: "#555" }}>
          workspace: {health.workspacePath ?? "(none)"}
        </div>

        <button
          data-testid="restart-sidecar-btn"
          onClick={async () => {
            setSidecarError(null);
            if (typeof (window.markbook as any).restartSidecar === "function") {
              await window.markbook.restartSidecar();
            }
            await refresh();
          }}
        >
          Restart Sidecar
        </button>
        <button data-testid="select-workspace-btn" onClick={() => void selectWorkspaceDialog()}>
          Select Workspace
        </button>
        <button
          data-testid="import-legacy-btn"
          onClick={() => void importLegacyClassFolder()}
        >
          Import Legacy Class Folder
        </button>
        <button data-testid="refresh-btn" onClick={refresh}>
          Refresh
        </button>

        {lastGridEvent ? (
          <div style={{ color: "#666", fontSize: 12 }}>grid: {lastGridEvent}</div>
        ) : null}
        {sidecarError ? (
          <div style={{ marginLeft: "auto", color: "#b00020" }}>{sidecarError}</div>
        ) : (
          <div style={{ marginLeft: "auto" }} />
        )}
      </div>

      <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
        <div style={{ width: 300, borderRight: "1px solid #ddd", padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Classes</div>
          {classes.length === 0 ? (
            <div style={{ color: "#666" }}>(none yet)</div>
          ) : (
            <ul data-testid="classes-list" style={{ margin: 0, paddingLeft: 18 }}>
              {classes.map((c) => (
                <li key={c.id}>
                  <button
                    data-testid={`class-btn-${c.id}`}
                    onClick={() => setSelectedClassId(c.id)}
                    style={{
                      border: "none",
                      background: "transparent",
                      padding: 0,
                      cursor: "pointer",
                      fontWeight: c.id === selectedClassId ? 700 : 400
                    }}
                    title={`Students: ${c.studentCount ?? "?"} | Mark sets: ${c.markSetCount ?? "?"}`}
                  >
                    {c.name}
                  </button>
                </li>
              ))}
            </ul>
          )}

          <div style={{ fontWeight: 700, marginTop: 16, marginBottom: 8 }}>Mark Sets</div>
          {!selectedClassId ? (
            <div style={{ color: "#666" }}>(select a class)</div>
          ) : markSets.length === 0 ? (
            <div style={{ color: "#666" }}>(none yet)</div>
          ) : (
            <ul data-testid="marksets-list" style={{ margin: 0, paddingLeft: 18 }}>
              {markSets.map((m) => (
                <li key={m.id}>
                  <button
                    data-testid={`markset-btn-${m.id}`}
                    onClick={() => setSelectedMarkSetId(m.id)}
                    style={{
                      border: "none",
                      background: "transparent",
                      padding: 0,
                      cursor: "pointer",
                      fontWeight: m.id === selectedMarkSetId ? 700 : 400
                    }}
                    title={m.description}
                  >
                    {m.code}: {m.description}
                  </button>
                </li>
              ))}
            </ul>
          )}

          <div style={{ fontWeight: 700, marginTop: 16, marginBottom: 8 }}>
            Class Tools
          </div>
          {!selectedClassId ? (
            <div style={{ color: "#666" }}>(select a class)</div>
          ) : (
            <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
              <button data-testid="nav-dashboard" onClick={() => setScreen("dashboard")}>
                Dashboard
              </button>
              <button
                data-testid="nav-class-wizard"
                onClick={() => {
                  setClassWizardMode("create");
                  setScreen("class_wizard");
                }}
              >
                New Class Wizard
              </button>
              <button
                data-testid="nav-class-profile"
                disabled={!selectedClassId}
                onClick={() => {
                  if (!selectedClassId) return;
                  setClassWizardMode("edit");
                  setScreen("class_wizard");
                }}
              >
                Class Profile
              </button>
              <button data-testid="nav-marks" onClick={() => setScreen("marks")}>
                Marks
              </button>
              <button data-testid="nav-students" onClick={() => setScreen("students")}>
                Students
              </button>
              <button
                data-testid="nav-markset-setup"
                onClick={() => setScreen("markset_setup")}
              >
                Mark Set Setup
              </button>
              <button data-testid="nav-attendance" onClick={() => setScreen("attendance")}>
                Attendance
              </button>
              <button data-testid="nav-notes" onClick={() => setScreen("notes")}>
                Notes
              </button>
              <button data-testid="nav-seating" onClick={() => setScreen("seating_plan")}>
                Seating Plan
              </button>
              <button
                data-testid="nav-learning-skills"
                onClick={() => setScreen("learning_skills")}
              >
                Learning Skills
              </button>
              <button data-testid="nav-loaned-items" onClick={() => setScreen("loaned_items")}>
                Loaned Items
              </button>
              <button
                data-testid="nav-device-mappings"
                onClick={() => setScreen("device_mappings")}
              >
                Device Mappings
              </button>
              <button
                data-testid="nav-calc-settings"
                onClick={() => setScreen("calc_settings")}
              >
                Calc Settings
              </button>
              <button data-testid="nav-backup" onClick={() => setScreen("backup")}>
                Backup
              </button>
              <button data-testid="nav-exchange" onClick={() => setScreen("exchange")}>
                Exchange
              </button>
              <button data-testid="nav-reports" onClick={() => setScreen("reports")}>
                Reports
              </button>
              <button
                data-testid="nav-class-analytics"
                onClick={() => setScreen("class_analytics")}
              >
                Class Analytics
              </button>
              <button
                data-testid="nav-student-analytics"
                onClick={() => setScreen("student_analytics")}
              >
                Student Analytics
              </button>
              <button
                data-testid="nav-combined-analytics"
                onClick={() => setScreen("combined_analytics")}
              >
                Combined Analytics
              </button>
              <button
                data-testid="delete-class-btn"
                onClick={() => selectedClassId && void deleteClass(selectedClassId)}
                style={{ color: "#b00020" }}
              >
                Delete Class
              </button>
            </div>
          )}

          <div style={{ fontWeight: 700, marginTop: 16, marginBottom: 8 }}>
            Legacy Menus
          </div>
          <div style={{ display: "flex", flexDirection: "column", gap: 8, fontSize: 13 }}>
            <details>
              <summary>File</summary>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 6 }}>
                <button
                  onClick={() => {
                    setClassWizardMode("create");
                    setScreen("class_wizard");
                  }}
                >
                  Make a New Class
                </button>
                <button
                  onClick={() => {
                    if (!selectedClassId) return;
                    setClassWizardMode("edit");
                    setScreen("class_wizard");
                  }}
                  disabled={!selectedClassId}
                >
                  Edit Class Profile
                </button>
                <button onClick={() => setScreen("dashboard")}>Open a Class</button>
                <button onClick={() => setScreen("backup")}>BackUp</button>
                <button onClick={() => setScreen("exchange")}>Exports</button>
                <button disabled title="Not implemented yet">
                  Select Printer
                </button>
              </div>
            </details>
            <details>
              <summary>Mark Sets</summary>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 6 }}>
                <button onClick={() => setScreen("markset_setup")}>Make a New Mark Set</button>
                <button onClick={() => setScreen("marks")}>Open a Mark Set</button>
                <button onClick={() => setScreen("markset_setup")}>Edit Heading and Categories</button>
                <button disabled title="Not implemented yet">
                  Undelete a Mark Set
                </button>
              </div>
            </details>
            <details>
              <summary>Working On</summary>
              <div style={{ display: "flex", flexDirection: "column", gap: 4, marginTop: 6 }}>
                <button onClick={() => setScreen("markset_setup")}>Entry Heading</button>
                <button onClick={() => setScreen("marks")}>Edit Marks</button>
                <button onClick={() => setScreen("reports")}>Display/Print</button>
                <button disabled title="Not implemented yet">
                  Clone Entry
                </button>
              </div>
            </details>
          </div>

          <div style={{ marginTop: 16, fontSize: 12, color: "#666" }}>
            Grid and reports are backed by SQLite via sidecar IPC.
          </div>
        </div>

        <div style={{ flex: 1, minWidth: 0 }}>
          {screen === "dashboard" ? (
            <DashboardScreen
              health={health}
              sidecarMeta={sidecarMeta}
              prefs={prefs}
              classes={classes}
              selectedClassId={selectedClassId}
              onSelectWorkspaceDialog={selectWorkspaceDialog}
              onOpenWorkspacePath={openWorkspacePath}
              onCreateClass={createClass}
              onDeleteClass={deleteClass}
              onImportLegacyClassFolder={importLegacyClassFolder}
              onNavigate={(s) => setScreen(s as Screen)}
              onOpenClassWizard={() => {
                setClassWizardMode("create");
                setScreen("class_wizard");
              }}
              onOpenClassProfile={(classId) => {
                setSelectedClassId(classId);
                setClassWizardMode("edit");
                setScreen("class_wizard");
              }}
              onPreviewLegacyUpdate={previewLegacyUpdate}
              onUpdateFromLegacy={updateFromLegacy}
              legacyPreviewByClass={legacyPreviewByClass}
            />
          ) : screen === "class_wizard" ? (
            <ClassWizardScreen
              onError={setSidecarError}
              onCancel={() => setScreen("dashboard")}
              mode={classWizardMode}
              selectedClassId={selectedClassId}
              onCreated={async (classId) => {
                setSelectedClassId(classId);
                setScreen("students");
                await refresh();
              }}
              onMetaSaved={async (classId) => {
                setSelectedClassId(classId);
                await refresh();
              }}
            />
          ) : !selectedClassId ? (
            <div style={{ padding: 24, color: "#666" }}>Select a class.</div>
          ) : !selectedMarkSetId &&
            (screen === "marks" ||
              screen === "reports" ||
              screen === "markset_setup" ||
              screen === "class_analytics" ||
              screen === "student_analytics") ? (
            <div style={{ padding: 24, color: "#666" }}>Select a mark set.</div>
          ) : screen === "marks" ? (
            <MarksScreen
              selectedClassId={selectedClassId}
              selectedMarkSetId={selectedMarkSetId as string}
              onError={setSidecarError}
              onGridEvent={(msg) => setLastGridEvent(msg)}
            />
          ) : screen === "reports" ? (
            <ReportsScreen
              selectedClassId={selectedClassId}
              selectedMarkSetId={selectedMarkSetId as string}
              onError={setSidecarError}
              initialContext={
                reportsPrefill
                  ? {
                      filters: reportsPrefill.filters,
                      studentScope: reportsPrefill.studentScope,
                      studentId: reportsPrefill.studentId ?? null,
                      markSetIds: reportsPrefill.markSetIds ?? null
                    }
                  : undefined
              }
              contextVersion={reportsPrefill?.nonce ?? 0}
            />
          ) : screen === "class_analytics" ? (
            <ClassAnalyticsScreen
              selectedClassId={selectedClassId}
              selectedMarkSetId={selectedMarkSetId as string}
              onError={setSidecarError}
              onOpenReports={(ctx) => {
                setReportsPrefill({
                  filters: ctx.filters,
                  studentScope: ctx.studentScope,
                  studentId: null,
                  markSetIds: null,
                  nonce: Date.now()
                });
                setScreen("reports");
              }}
            />
          ) : screen === "student_analytics" ? (
            <StudentAnalyticsScreen
              selectedClassId={selectedClassId}
              selectedMarkSetId={selectedMarkSetId as string}
              onError={setSidecarError}
              onOpenReports={(ctx) => {
                setReportsPrefill({
                  filters: ctx.filters,
                  studentScope: ctx.studentScope,
                  studentId: ctx.studentId ?? null,
                  markSetIds: null,
                  nonce: Date.now()
                });
                setScreen("reports");
              }}
            />
          ) : screen === "combined_analytics" ? (
            <CombinedAnalyticsScreen
              selectedClassId={selectedClassId}
              onError={setSidecarError}
              onOpenReports={(ctx) => {
                setReportsPrefill({
                  filters: ctx.filters,
                  studentScope: ctx.studentScope,
                  studentId: null,
                  markSetIds: ctx.markSetIds,
                  nonce: Date.now()
                });
                setScreen("reports");
              }}
            />
          ) : screen === "students" ? (
            <StudentsScreen
              selectedClassId={selectedClassId}
              onError={setSidecarError}
              onChanged={refresh}
            />
          ) : screen === "markset_setup" ? (
            <MarkSetSetupScreen
              selectedClassId={selectedClassId}
              selectedMarkSetId={selectedMarkSetId as string}
              onError={setSidecarError}
              onChanged={refresh}
              onSelectMarkSet={(markSetId) => setSelectedMarkSetId(markSetId)}
            />
          ) : screen === "attendance" ? (
            <AttendanceScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "notes" ? (
            <NotesScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "seating_plan" ? (
            <SeatingPlanScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "learning_skills" ? (
            <LearningSkillsScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "calc_settings" ? (
            <CalcSettingsScreen onError={setSidecarError} />
          ) : screen === "loaned_items" ? (
            <LoanedItemsScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "device_mappings" ? (
            <DeviceMappingsScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : screen === "backup" ? (
            <BackupScreen
              workspacePath={health.workspacePath}
              onError={setSidecarError}
              onAfterImport={refresh}
            />
          ) : screen === "exchange" ? (
            <ExchangeScreen selectedClassId={selectedClassId} onError={setSidecarError} />
          ) : (
            <div style={{ padding: 24, color: "#666" }}>(unknown screen)</div>
          )}
        </div>
      </div>
    </div>
  );
}
