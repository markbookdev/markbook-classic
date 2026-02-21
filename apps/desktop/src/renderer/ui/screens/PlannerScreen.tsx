import React, { useEffect, useMemo, useState } from "react";
import {
  PlannerLessonsArchiveResultSchema,
  PlannerLessonsCreateResultSchema,
  PlannerLessonsListResultSchema,
  PlannerLessonsReorderResultSchema,
  PlannerLessonsUpdateResultSchema,
  PlannerPublishCommitResultSchema,
  PlannerPublishListResultSchema,
  PlannerPublishPreviewResultSchema,
  PlannerPublishUpdateStatusResultSchema,
  PlannerUnitsArchiveResultSchema,
  PlannerUnitsCreateResultSchema,
  PlannerUnitsListResultSchema,
  PlannerUnitsReorderResultSchema,
  PlannerUnitsUpdateResultSchema,
  SetupGetResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type PlannerTab = "units" | "lessons" | "publish";
type ArtifactKind = "unit" | "lesson" | "course_description" | "time_management";

type PlannerDefaults = {
  defaultLessonDurationMinutes: number;
  defaultPublishStatus: "draft" | "published" | "archived";
  showArchivedByDefault: boolean;
  defaultUnitTitlePrefix: string;
};

const DEFAULTS: PlannerDefaults = {
  defaultLessonDurationMinutes: 75,
  defaultPublishStatus: "draft",
  showArchivedByDefault: false,
  defaultUnitTitlePrefix: "Unit"
};

function moveByOne(ids: string[], id: string, delta: -1 | 1): string[] {
  const idx = ids.indexOf(id);
  if (idx < 0) return ids;
  const next = idx + delta;
  if (next < 0 || next >= ids.length) return ids;
  const out = [...ids];
  const t = out[idx];
  out[idx] = out[next];
  out[next] = t;
  return out;
}

export function PlannerScreen(props: { selectedClassId: string; onError: (msg: string | null) => void }) {
  const [tab, setTab] = useState<PlannerTab>("units");
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [defaults, setDefaults] = useState<PlannerDefaults>(DEFAULTS);
  const [includeArchived, setIncludeArchived] = useState(false);

  const [units, setUnits] = useState<
    Array<{
      id: string;
      sortOrder: number;
      title: string;
      startDate: string | null;
      endDate: string | null;
      summary: string;
      archived: boolean;
    }>
  >([]);
  const [lessons, setLessons] = useState<
    Array<{
      id: string;
      unitId: string | null;
      sortOrder: number;
      lessonDate: string | null;
      title: string;
      durationMinutes: number | null;
      archived: boolean;
    }>
  >([]);
  const [published, setPublished] = useState<
    Array<{
      id: string;
      artifactKind: ArtifactKind;
      sourceId: string | null;
      title: string;
      status: "draft" | "published" | "archived";
      version: number;
      updatedAt: string;
    }>
  >([]);

  const [selectedUnitId, setSelectedUnitId] = useState<string | null>(null);
  const [selectedLessonId, setSelectedLessonId] = useState<string | null>(null);
  const [lessonUnitFilter, setLessonUnitFilter] = useState<string | null>(null);

  const [newUnitTitle, setNewUnitTitle] = useState("");
  const [newLessonTitle, setNewLessonTitle] = useState("");
  const [newLessonDate, setNewLessonDate] = useState<string>("");
  const [newLessonUnitId, setNewLessonUnitId] = useState<string>("");
  const [newLessonDuration, setNewLessonDuration] = useState<string>(String(DEFAULTS.defaultLessonDurationMinutes));

  const [publishArtifactKind, setPublishArtifactKind] = useState<ArtifactKind>("unit");
  const [publishTitle, setPublishTitle] = useState("");
  const [publishStatus, setPublishStatus] = useState<"draft" | "published" | "archived">("draft");
  const [publishPreview, setPublishPreview] = useState<any>(null);
  const [publishMessage, setPublishMessage] = useState<string | null>(null);

  const filteredLessons = useMemo(() => {
    if (!lessonUnitFilter || lessonUnitFilter === "ALL") return lessons;
    return lessons.filter((lesson) => lesson.unitId === lessonUnitFilter);
  }, [lessonUnitFilter, lessons]);

  useEffect(() => {
    void loadAll();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId, includeArchived]);

  useEffect(() => {
    let cancelled = false;
    async function loadDefaults() {
      try {
        const setupRes = await requestParsed("setup.get", {}, SetupGetResultSchema);
        if (cancelled) return;
        setDefaults({
          defaultLessonDurationMinutes: setupRes.planner.defaultLessonDurationMinutes,
          defaultPublishStatus: setupRes.planner.defaultPublishStatus,
          showArchivedByDefault: setupRes.planner.showArchivedByDefault,
          defaultUnitTitlePrefix: setupRes.planner.defaultUnitTitlePrefix
        });
        setIncludeArchived(setupRes.planner.showArchivedByDefault);
        setNewLessonDuration(String(setupRes.planner.defaultLessonDurationMinutes));
        setPublishStatus(setupRes.planner.defaultPublishStatus);
      } catch (e: any) {
        if (cancelled) return;
        props.onError(e?.message ?? String(e));
      }
    }
    void loadDefaults();
    return () => {
      cancelled = true;
    };
  }, [props.selectedClassId]);

  async function loadAll() {
    setLoading(true);
    props.onError(null);
    try {
      const [unitsRes, lessonsRes, publishRes] = await Promise.all([
        requestParsed(
          "planner.units.list",
          { classId: props.selectedClassId, includeArchived },
          PlannerUnitsListResultSchema
        ),
        requestParsed(
          "planner.lessons.list",
          { classId: props.selectedClassId, includeArchived },
          PlannerLessonsListResultSchema
        ),
        requestParsed(
          "planner.publish.list",
          { classId: props.selectedClassId },
          PlannerPublishListResultSchema
        )
      ]);

      setUnits(unitsRes.units);
      setSelectedUnitId((cur) => {
        if (cur && unitsRes.units.some((u) => u.id === cur)) return cur;
        return unitsRes.units[0]?.id ?? null;
      });

      setLessons(lessonsRes.lessons);
      setSelectedLessonId((cur) => {
        if (cur && lessonsRes.lessons.some((l) => l.id === cur)) return cur;
        return lessonsRes.lessons[0]?.id ?? null;
      });

      setPublished(publishRes.published as any);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }

  async function createUnit() {
    const fallbackTitle = `${(defaults.defaultUnitTitlePrefix || "Unit").trim() || "Unit"} ${Math.max(
      units.length + 1,
      1
    )}`;
    const title = (newUnitTitle.trim() || fallbackTitle).trim();
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.units.create",
        { classId: props.selectedClassId, input: { title } },
        PlannerUnitsCreateResultSchema
      );
      setNewUnitTitle("");
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function renameUnit(unitId: string, currentTitle: string) {
    const nextTitle = prompt("Unit title", currentTitle)?.trim();
    if (!nextTitle || nextTitle === currentTitle) return;
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.units.update",
        { classId: props.selectedClassId, unitId, patch: { title: nextTitle } },
        PlannerUnitsUpdateResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function reorderUnits(unitId: string, delta: -1 | 1) {
    const nextOrder = moveByOne(
      units
        .slice()
        .sort((a, b) => a.sortOrder - b.sortOrder || a.title.localeCompare(b.title))
        .map((u) => u.id),
      unitId,
      delta
    );
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.units.reorder",
        { classId: props.selectedClassId, unitIds: nextOrder },
        PlannerUnitsReorderResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function toggleUnitArchive(unitId: string, archived: boolean) {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.units.archive",
        { classId: props.selectedClassId, unitId, archived },
        PlannerUnitsArchiveResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function createLesson() {
    const title = newLessonTitle.trim();
    if (!title) return;
    const duration = Number.parseInt(newLessonDuration.trim(), 10);
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.lessons.create",
        {
          classId: props.selectedClassId,
          input: {
            title,
            unitId: newLessonUnitId || null,
            lessonDate: newLessonDate.trim() || null,
            durationMinutes: Number.isFinite(duration) && duration > 0 ? duration : defaults.defaultLessonDurationMinutes
          }
        },
        PlannerLessonsCreateResultSchema
      );
      setNewLessonTitle("");
      setNewLessonDate("");
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function renameLesson(lessonId: string, currentTitle: string) {
    const nextTitle = prompt("Lesson title", currentTitle)?.trim();
    if (!nextTitle || nextTitle === currentTitle) return;
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.lessons.update",
        { classId: props.selectedClassId, lessonId, patch: { title: nextTitle } },
        PlannerLessonsUpdateResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function reorderLessons(lessonId: string, delta: -1 | 1) {
    const ordered = filteredLessons
      .slice()
      .sort((a, b) => a.sortOrder - b.sortOrder || a.title.localeCompare(b.title))
      .map((l) => l.id);
    const nextOrder = moveByOne(ordered, lessonId, delta);
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.lessons.reorder",
        {
          classId: props.selectedClassId,
          lessonIdOrder: nextOrder,
          unitId: lessonUnitFilter && lessonUnitFilter !== "ALL" ? lessonUnitFilter : null
        },
        PlannerLessonsReorderResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function toggleLessonArchive(lessonId: string, archived: boolean) {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.lessons.archive",
        { classId: props.selectedClassId, lessonId, archived },
        PlannerLessonsArchiveResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  function resolvedPublishSourceId(): string | null {
    if (publishArtifactKind === "unit") return selectedUnitId;
    if (publishArtifactKind === "lesson") return selectedLessonId;
    return null;
  }

  async function previewPublishModel() {
    setPublishMessage(null);
    setSaving(true);
    props.onError(null);
    try {
      const sourceId = resolvedPublishSourceId();
      const preview = await requestParsed(
        "planner.publish.preview",
        {
          classId: props.selectedClassId,
          artifactKind: publishArtifactKind,
          sourceId,
          options: {}
        },
        PlannerPublishPreviewResultSchema
      );
      setPublishPreview(preview);
      if (!publishTitle.trim()) {
        setPublishTitle(preview.title);
      }
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function commitPublish() {
    const title = publishTitle.trim();
    if (!title) return;
    setPublishMessage(null);
    setSaving(true);
    props.onError(null);
    try {
      const sourceId = resolvedPublishSourceId();
      await requestParsed(
        "planner.publish.commit",
        {
          classId: props.selectedClassId,
          artifactKind: publishArtifactKind,
          sourceId,
          title,
          model: publishPreview?.model ?? {},
          status: publishStatus
        },
        PlannerPublishCommitResultSchema
      );
      setPublishMessage("Published artifact saved.");
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function updatePublishStatus(id: string, status: "draft" | "published" | "archived") {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "planner.publish.updateStatus",
        { classId: props.selectedClassId, publishId: id, status },
        PlannerPublishUpdateStatusResultSchema
      );
      await loadAll();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div data-testid="planner-screen" style={{ padding: 24 }}>
      <div style={{ fontSize: 22, fontWeight: 800, marginBottom: 10 }}>Planner</div>
      <div style={{ display: "flex", gap: 8, marginBottom: 12 }}>
        <button
          data-testid="planner-units-tab"
          onClick={() => setTab("units")}
          style={{ fontWeight: tab === "units" ? 700 : 400 }}
        >
          Units
        </button>
        <button
          data-testid="planner-lessons-tab"
          onClick={() => setTab("lessons")}
          style={{ fontWeight: tab === "lessons" ? 700 : 400 }}
        >
          Lessons
        </button>
        <button
          data-testid="planner-publish-tab"
          onClick={() => setTab("publish")}
          style={{ fontWeight: tab === "publish" ? 700 : 400 }}
        >
          Publish
        </button>
        <label style={{ marginLeft: 16, display: "flex", gap: 6, alignItems: "center" }}>
          <input
            data-testid="planner-show-archived-toggle"
            type="checkbox"
            checked={includeArchived}
            onChange={(e) => setIncludeArchived(e.currentTarget.checked)}
          />
          Show archived
        </label>
      </div>

      {tab === "units" ? (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 10 }}>
            <input
              placeholder="New unit title"
              value={newUnitTitle}
              onChange={(e) => setNewUnitTitle(e.currentTarget.value)}
              style={{ width: 320 }}
            />
            <button
              data-testid="planner-unit-create-btn"
              onClick={() => void createUnit()}
              disabled={saving || loading || !newUnitTitle.trim()}
            >
              Create Unit
            </button>
          </div>
          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>Title</th>
                <th>Start</th>
                <th>End</th>
                <th>Archived</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {units.map((unit) => (
                <tr key={unit.id}>
                  <td>{unit.title}</td>
                  <td>{unit.startDate ?? ""}</td>
                  <td>{unit.endDate ?? ""}</td>
                  <td>{unit.archived ? "Y" : "N"}</td>
                  <td style={{ display: "flex", gap: 6 }}>
                    <button onClick={() => setSelectedUnitId(unit.id)}>Select</button>
                    <button onClick={() => void renameUnit(unit.id, unit.title)}>Rename</button>
                    <button onClick={() => void reorderUnits(unit.id, -1)}>Up</button>
                    <button onClick={() => void reorderUnits(unit.id, 1)}>Down</button>
                    <button onClick={() => void toggleUnitArchive(unit.id, !unit.archived)}>
                      {unit.archived ? "Unarchive" : "Archive"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      {tab === "lessons" ? (
        <div>
          <div style={{ display: "flex", gap: 8, marginBottom: 10, flexWrap: "wrap" }}>
            <input
              placeholder="New lesson title"
              value={newLessonTitle}
              onChange={(e) => setNewLessonTitle(e.currentTarget.value)}
              style={{ width: 280 }}
            />
            <input
              type="date"
              value={newLessonDate}
              onChange={(e) => setNewLessonDate(e.currentTarget.value)}
            />
            <input
              value={newLessonDuration}
              onChange={(e) => setNewLessonDuration(e.currentTarget.value)}
              style={{ width: 100 }}
              placeholder="Minutes"
            />
            <select
              value={newLessonUnitId}
              onChange={(e) => setNewLessonUnitId(e.currentTarget.value)}
            >
              <option value="">No Unit</option>
              {units.map((unit) => (
                <option key={unit.id} value={unit.id}>
                  {unit.title}
                </option>
              ))}
            </select>
            <button
              data-testid="planner-lesson-create-btn"
              onClick={() => void createLesson()}
              disabled={saving || loading || !newLessonTitle.trim()}
            >
              Create Lesson
            </button>
          </div>

          <div style={{ marginBottom: 8 }}>
            <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
              Unit Filter
              <select
                value={lessonUnitFilter ?? "ALL"}
                onChange={(e) => setLessonUnitFilter(e.currentTarget.value)}
              >
                <option value="ALL">ALL</option>
                {units.map((unit) => (
                  <option key={unit.id} value={unit.id}>
                    {unit.title}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <table style={{ width: "100%", borderCollapse: "collapse" }}>
            <thead>
              <tr>
                <th style={{ textAlign: "left" }}>Title</th>
                <th>Date</th>
                <th>Unit</th>
                <th>Minutes</th>
                <th>Archived</th>
                <th>Actions</th>
              </tr>
            </thead>
            <tbody>
              {filteredLessons.map((lesson) => (
                <tr key={lesson.id}>
                  <td>{lesson.title}</td>
                  <td>{lesson.lessonDate ?? ""}</td>
                  <td>{units.find((u) => u.id === lesson.unitId)?.title ?? ""}</td>
                  <td>{lesson.durationMinutes ?? ""}</td>
                  <td>{lesson.archived ? "Y" : "N"}</td>
                  <td style={{ display: "flex", gap: 6 }}>
                    <button onClick={() => setSelectedLessonId(lesson.id)}>Select</button>
                    <button onClick={() => void renameLesson(lesson.id, lesson.title)}>Rename</button>
                    <button onClick={() => void reorderLessons(lesson.id, -1)}>Up</button>
                    <button onClick={() => void reorderLessons(lesson.id, 1)}>Down</button>
                    <button onClick={() => void toggleLessonArchive(lesson.id, !lesson.archived)}>
                      {lesson.archived ? "Unarchive" : "Archive"}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      ) : null}

      {tab === "publish" ? (
        <div>
          <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 10 }}>
            <label>
              Artifact
              <select
                value={publishArtifactKind}
                onChange={(e) => setPublishArtifactKind(e.currentTarget.value as ArtifactKind)}
                style={{ marginLeft: 6 }}
              >
                <option value="unit">unit</option>
                <option value="lesson">lesson</option>
                <option value="course_description">course_description</option>
                <option value="time_management">time_management</option>
              </select>
            </label>

            {publishArtifactKind === "unit" ? (
              <label>
                Unit
                <select
                  value={selectedUnitId ?? ""}
                  onChange={(e) => setSelectedUnitId(e.currentTarget.value || null)}
                  style={{ marginLeft: 6 }}
                >
                  {units.map((unit) => (
                    <option key={unit.id} value={unit.id}>
                      {unit.title}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            {publishArtifactKind === "lesson" ? (
              <label>
                Lesson
                <select
                  value={selectedLessonId ?? ""}
                  onChange={(e) => setSelectedLessonId(e.currentTarget.value || null)}
                  style={{ marginLeft: 6 }}
                >
                  {lessons.map((lesson) => (
                    <option key={lesson.id} value={lesson.id}>
                      {lesson.title}
                    </option>
                  ))}
                </select>
              </label>
            ) : null}

            <button
              data-testid="planner-publish-preview-btn"
              onClick={() => void previewPublishModel()}
              disabled={saving || loading}
            >
              Preview
            </button>
          </div>

          <div style={{ display: "flex", gap: 8, alignItems: "center", marginBottom: 10 }}>
            <input
              placeholder="Publish title"
              value={publishTitle}
              onChange={(e) => setPublishTitle(e.currentTarget.value)}
              style={{ width: 300 }}
            />
            <select
              value={publishStatus}
              onChange={(e) => setPublishStatus(e.currentTarget.value as any)}
            >
              <option value="draft">draft</option>
              <option value="published">published</option>
              <option value="archived">archived</option>
            </select>
            <button
              data-testid="planner-publish-commit-btn"
              onClick={() => void commitPublish()}
              disabled={saving || loading || !publishTitle.trim()}
            >
              Commit
            </button>
          </div>

          {publishMessage ? <div style={{ marginBottom: 8, color: "#2d7" }}>{publishMessage}</div> : null}

          <div style={{ marginBottom: 10 }}>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>Preview Model</div>
            <pre style={{ maxHeight: 220, overflow: "auto", background: "#f7f7f7", padding: 10 }}>
              {publishPreview ? JSON.stringify(publishPreview, null, 2) : "(none)"}
            </pre>
          </div>

          <div>
            <div style={{ fontWeight: 600, marginBottom: 6 }}>Published Artifacts</div>
            <table style={{ width: "100%", borderCollapse: "collapse" }}>
              <thead>
                <tr>
                  <th style={{ textAlign: "left" }}>Title</th>
                  <th>Kind</th>
                  <th>Version</th>
                  <th>Status</th>
                  <th>Updated</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {published.map((row) => (
                  <tr key={row.id}>
                    <td>{row.title}</td>
                    <td>{row.artifactKind}</td>
                    <td>{row.version}</td>
                    <td>{row.status}</td>
                    <td>{row.updatedAt}</td>
                    <td style={{ display: "flex", gap: 6, justifyContent: "center" }}>
                      <button onClick={() => void updatePublishStatus(row.id, "draft")}>Draft</button>
                      <button onClick={() => void updatePublishStatus(row.id, "published")}>Publish</button>
                      <button onClick={() => void updatePublishStatus(row.id, "archived")}>Archive</button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      ) : null}
    </div>
  );
}
