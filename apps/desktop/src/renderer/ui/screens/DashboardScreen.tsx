import React, { useMemo, useState } from "react";
import type { HealthState, Prefs, SidecarMeta } from "../state/workspace";

type ClassRow = {
  id: string;
  name: string;
  studentCount?: number;
  markSetCount?: number;
};

export function DashboardScreen(props: {
  health: HealthState;
  sidecarMeta: SidecarMeta | null;
  prefs: Prefs | null;
  classes: ClassRow[];
  selectedClassId: string | null;
  onSelectWorkspaceDialog: () => Promise<void>;
  onOpenWorkspacePath: (path: string) => Promise<void>;
  onCreateClass: (name: string) => Promise<void>;
  onDeleteClass: (classId: string) => Promise<void>;
  onImportLegacyClassFolder: () => Promise<void>;
  onNavigate: (screen: string) => void;
  onOpenClassWizard: () => void;
  onOpenClassProfile: (classId: string) => void;
  onPreviewLegacyUpdate: (classId: string) => Promise<void>;
  onUpdateFromLegacy: (classId: string) => Promise<void>;
  legacyPreviewByClass?: { classId: string; data: any } | null;
}) {
  const [newClassName, setNewClassName] = useState("");

  const selectedClass = useMemo(
    () => props.classes.find((c) => c.id === props.selectedClassId) ?? null,
    [props.classes, props.selectedClassId]
  );
  const selectedPreview =
    selectedClass &&
    props.legacyPreviewByClass &&
    props.legacyPreviewByClass.classId === selectedClass.id
      ? props.legacyPreviewByClass.data
      : null;

  return (
    <div style={{ padding: 24, maxWidth: 920 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 12 }}>
        Dashboard
      </div>

      <div style={{ display: "flex", gap: 16, flexWrap: "wrap" }}>
        <div
          style={{
            flex: "1 1 420px",
            border: "1px solid #ddd",
            borderRadius: 10,
            padding: 16
          }}
        >
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Workspace</div>
          <div style={{ fontSize: 13, color: "#444", lineHeight: 1.35 }}>
            <div>
              <strong>Status:</strong> {props.health.sidecar ? "Sidecar OK" : "Sidecar down"}
            </div>
            <div>
              <strong>Version:</strong> {props.health.version}
            </div>
            <div>
              <strong>Workspace:</strong>{" "}
              {props.health.workspacePath ?? "(none selected)"}
            </div>
            <div>
              <strong>Sidecar bin:</strong>{" "}
              {props.sidecarMeta?.path ?? "(unknown)"}
            </div>
          </div>
          <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
            <button onClick={() => void props.onSelectWorkspaceDialog()}>
              Select Workspace
            </button>
            <button onClick={() => void props.onImportLegacyClassFolder()}>
              Import Legacy Class Folder
            </button>
          </div>
          {props.prefs?.recentWorkspaces?.length ? (
            <div style={{ marginTop: 14 }}>
              <div style={{ fontWeight: 700, fontSize: 12, color: "#444", marginBottom: 6 }}>
                Recent
              </div>
              <ul style={{ margin: 0, paddingLeft: 18 }}>
                {props.prefs.recentWorkspaces.map((p) => (
                  <li key={p} style={{ marginBottom: 4 }}>
                    <button
                      style={{ border: "none", background: "transparent", padding: 0, cursor: "pointer", color: "#0b57d0" }}
                      onClick={() => void props.onOpenWorkspacePath(p)}
                      title={p}
                    >
                      {p}
                    </button>
                  </li>
                ))}
              </ul>
            </div>
          ) : null}
        </div>

        <div
          style={{
            flex: "1 1 420px",
            border: "1px solid #ddd",
            borderRadius: 10,
            padding: 16
          }}
        >
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Classes</div>
          {props.classes.length === 0 ? (
            <div style={{ color: "#666" }}>(none yet)</div>
          ) : (
            <div style={{ fontSize: 13, color: "#444", lineHeight: 1.35 }}>
              <div>
                <strong>Total:</strong> {props.classes.length}
              </div>
              {selectedClass ? (
                <div style={{ marginTop: 8 }}>
                  <div style={{ fontWeight: 700 }}>Selected</div>
                  <div>{selectedClass.name}</div>
                  <div style={{ color: "#666" }}>
                    Students: {selectedClass.studentCount ?? "?"} | Mark sets:{" "}
                    {selectedClass.markSetCount ?? "?"}
                  </div>
                  <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                    <button onClick={() => props.onNavigate("marks")}>Open Marks</button>
                    <button onClick={() => props.onNavigate("students")}>Students</button>
                    <button onClick={() => props.onNavigate("markset_setup")}>Mark Set Setup</button>
                    <button onClick={() => props.onNavigate("reports")}>Reports</button>
                    <button
                      data-testid="dashboard-open-class-profile-btn"
                      onClick={() => props.onOpenClassProfile(selectedClass.id)}
                    >
                      Class Profile
                    </button>
                  </div>
                  <div style={{ display: "flex", gap: 8, marginTop: 10 }}>
                    <button
                      data-testid="class-legacy-preview-btn"
                      onClick={() => void props.onPreviewLegacyUpdate(selectedClass.id)}
                    >
                      Preview Legacy Update
                    </button>
                    <button
                      data-testid="class-update-from-legacy-btn"
                      onClick={() => void props.onUpdateFromLegacy(selectedClass.id)}
                    >
                      Update From Legacy Folder
                    </button>
                    <div style={{ alignSelf: "center", fontSize: 12, color: "#666" }}>
                      Default mode: Upsert Preserve
                    </div>
                  </div>
                  {selectedPreview ? (
                    <div
                      data-testid="class-update-preview-summary"
                      style={{
                        marginTop: 10,
                        border: "1px solid #eee",
                        borderRadius: 8,
                        background: "#fafafa",
                        padding: 10,
                        fontSize: 12,
                        color: "#333"
                      }}
                    >
                      <div>
                        <strong>Preview:</strong> {selectedPreview.className ?? "Legacy class"}
                      </div>
                      <div>
                        Students: incoming {selectedPreview.students?.incoming ?? 0}, matched{" "}
                        {selectedPreview.students?.matched ?? 0}, new{" "}
                        {selectedPreview.students?.new ?? 0}, ambiguous{" "}
                        {selectedPreview.students?.ambiguous ?? 0}, local-only{" "}
                        {selectedPreview.students?.localOnly ?? 0}
                      </div>
                      <div>
                        Mark sets: incoming {selectedPreview.markSets?.incoming ?? 0}, matched{" "}
                        {selectedPreview.markSets?.matched ?? 0}, new{" "}
                        {selectedPreview.markSets?.new ?? 0}
                      </div>
                      <div>
                        Warnings:{" "}
                        {Array.isArray(selectedPreview.warnings)
                          ? selectedPreview.warnings.length
                          : 0}
                      </div>
                    </div>
                  ) : null}
                  <div style={{ marginTop: 10 }}>
                    <button
                      onClick={() => void props.onDeleteClass(selectedClass.id)}
                      style={{ color: "#b00020" }}
                    >
                      Delete Class
                    </button>
                  </div>
                </div>
              ) : (
                <div style={{ marginTop: 8, color: "#666" }}>(select a class)</div>
              )}
            </div>
          )}

          <div style={{ marginTop: 16, borderTop: "1px solid #eee", paddingTop: 12 }}>
            <div style={{ fontWeight: 700, marginBottom: 6 }}>Create Class</div>
            <div style={{ display: "flex", gap: 8 }}>
              <button data-testid="dashboard-open-class-wizard-btn" onClick={() => props.onOpenClassWizard()}>
                New Class Wizard
              </button>
              <input
                value={newClassName}
                onChange={(e) => setNewClassName(e.currentTarget.value)}
                placeholder="e.g. 8D (2026)"
                style={{ flex: 1, padding: "6px 8px" }}
              />
              <button
                onClick={() => {
                  const name = newClassName.trim();
                  if (!name) return;
                  setNewClassName("");
                  void props.onCreateClass(name);
                }}
              >
                Create
              </button>
            </div>
            <div style={{ marginTop: 8, fontSize: 12, color: "#666" }}>
              Creates an empty class in the current workspace.
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
