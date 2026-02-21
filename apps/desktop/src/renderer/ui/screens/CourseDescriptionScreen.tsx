import React, { useEffect, useState } from "react";
import {
  CourseDescriptionGetProfileResultSchema,
  CourseDescriptionModelResultSchema,
  CourseDescriptionUpdateProfileResultSchema,
  SetupGetResultSchema,
  TimeManagementModelResultSchema
} from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type ProfileState = {
  courseTitle: string;
  gradeLabel: string;
  periodMinutes: number;
  periodsPerWeek: number;
  totalWeeks: number;
  strands: string[];
  policyText: string;
  updatedAt: string | null;
};

const DEFAULT_PROFILE: ProfileState = {
  courseTitle: "",
  gradeLabel: "",
  periodMinutes: 75,
  periodsPerWeek: 5,
  totalWeeks: 36,
  strands: [],
  policyText: "",
  updatedAt: null
};

export function CourseDescriptionScreen(props: {
  selectedClassId: string;
  onError: (msg: string | null) => void;
}) {
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [profile, setProfile] = useState<ProfileState>(DEFAULT_PROFILE);
  const [includePolicyDefault, setIncludePolicyDefault] = useState(true);
  const [includeStrands, setIncludeStrands] = useState(true);
  const [includeAssessmentPlan, setIncludeAssessmentPlan] = useState(true);
  const [includeResources, setIncludeResources] = useState(true);
  const [generatedModel, setGeneratedModel] = useState<any>(null);
  const [timeManagementModel, setTimeManagementModel] = useState<any>(null);

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [props.selectedClassId]);

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const [profileRes, setupRes] = await Promise.all([
        requestParsed(
          "courseDescription.getProfile",
          { classId: props.selectedClassId },
          CourseDescriptionGetProfileResultSchema
        ),
        requestParsed("setup.get", {}, SetupGetResultSchema)
      ]);
      setProfile({
        ...profileRes.profile,
        periodMinutes:
          profileRes.profile.periodMinutes > 0
            ? profileRes.profile.periodMinutes
            : setupRes.courseDescription.defaultPeriodMinutes,
        periodsPerWeek:
          profileRes.profile.periodsPerWeek > 0
            ? profileRes.profile.periodsPerWeek
            : setupRes.courseDescription.defaultPeriodsPerWeek,
        totalWeeks:
          profileRes.profile.totalWeeks > 0
            ? profileRes.profile.totalWeeks
            : setupRes.courseDescription.defaultTotalWeeks
      });
      setIncludePolicyDefault(setupRes.courseDescription.includePolicyByDefault);
      setIncludeStrands(true);
      setIncludeAssessmentPlan(true);
      setIncludeResources(true);
      setGeneratedModel(null);
      setTimeManagementModel(null);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }

  async function saveProfile() {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "courseDescription.updateProfile",
        {
          classId: props.selectedClassId,
          patch: {
            courseTitle: profile.courseTitle,
            gradeLabel: profile.gradeLabel,
            periodMinutes: profile.periodMinutes,
            periodsPerWeek: profile.periodsPerWeek,
            totalWeeks: profile.totalWeeks,
            strands: profile.strands,
            policyText: profile.policyText
          }
        },
        CourseDescriptionUpdateProfileResultSchema
      );
      await load();
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function generateCourseDescription() {
    setSaving(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "courseDescription.generateModel",
        {
          classId: props.selectedClassId,
          options: {
            includePolicy: includePolicyDefault,
            includeStrands,
            includeAssessmentPlan,
            includeResources
          }
        },
        CourseDescriptionModelResultSchema
      );
      setGeneratedModel(model);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function generateTimeManagement() {
    setSaving(true);
    props.onError(null);
    try {
      const model = await requestParsed(
        "courseDescription.timeManagementModel",
        {
          classId: props.selectedClassId,
          options: {
            includeArchived: false
          }
        },
        TimeManagementModelResultSchema
      );
      setTimeManagementModel(model);
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  function parseIntSafe(value: string, fallback: number, min: number, max: number) {
    const parsed = Number.parseInt(value.trim(), 10);
    if (!Number.isFinite(parsed)) return fallback;
    return Math.max(min, Math.min(max, parsed));
  }

  const generationDiffSummary = (() => {
    if (!generatedModel || !generatedModel.profile) return "(no generated model yet)";
    const diffs: string[] = [];
    const gProfile = generatedModel.profile ?? {};
    if ((gProfile.courseTitle ?? "") !== profile.courseTitle) {
      diffs.push("courseTitle");
    }
    if ((gProfile.gradeLabel ?? "") !== profile.gradeLabel) {
      diffs.push("gradeLabel");
    }
    if (Number(gProfile.periodMinutes ?? 0) !== Number(profile.periodMinutes)) {
      diffs.push("periodMinutes");
    }
    if (Number(gProfile.periodsPerWeek ?? 0) !== Number(profile.periodsPerWeek)) {
      diffs.push("periodsPerWeek");
    }
    if (Number(gProfile.totalWeeks ?? 0) !== Number(profile.totalWeeks)) {
      diffs.push("totalWeeks");
    }
    const currentStrands = profile.strands.join("|");
    const generatedStrands = Array.isArray(gProfile.strands)
      ? gProfile.strands.map((v: any) => String(v ?? "")).join("|")
      : "";
    if (generatedStrands !== currentStrands) {
      diffs.push("strands");
    }
    if ((gProfile.policyText ?? "") !== profile.policyText) {
      diffs.push("policyText");
    }
    if (diffs.length === 0) return "Profile and generated model are aligned.";
    return `Differences from profile: ${diffs.join(", ")}`;
  })();

  return (
    <div data-testid="course-description-screen" style={{ padding: 24 }}>
      <div style={{ fontSize: 22, fontWeight: 800, marginBottom: 10 }}>Course Description</div>
      <div style={{ color: "#666", marginBottom: 12 }}>
        Manage class profile defaults and generate course-description/time-management models.
      </div>

      <div style={{ display: "grid", gridTemplateColumns: "repeat(auto-fit, minmax(260px, 1fr))", gap: 10 }}>
        <label>
          Course title
          <input
            value={profile.courseTitle}
            onChange={(e) => setProfile((p) => ({ ...p, courseTitle: e.currentTarget.value }))}
            style={{ display: "block", marginTop: 4, width: "100%" }}
          />
        </label>
        <label>
          Grade label
          <input
            value={profile.gradeLabel}
            onChange={(e) => setProfile((p) => ({ ...p, gradeLabel: e.currentTarget.value }))}
            style={{ display: "block", marginTop: 4, width: "100%" }}
          />
        </label>
        <label>
          Period minutes
          <input
            value={String(profile.periodMinutes)}
            onChange={(e) =>
              setProfile((p) => ({
                ...p,
                periodMinutes: parseIntSafe(e.currentTarget.value, p.periodMinutes, 1, 300)
              }))
            }
            style={{ display: "block", marginTop: 4, width: 120 }}
          />
        </label>
        <label>
          Periods per week
          <input
            value={String(profile.periodsPerWeek)}
            onChange={(e) =>
              setProfile((p) => ({
                ...p,
                periodsPerWeek: parseIntSafe(e.currentTarget.value, p.periodsPerWeek, 1, 14)
              }))
            }
            style={{ display: "block", marginTop: 4, width: 120 }}
          />
        </label>
        <label>
          Total weeks
          <input
            value={String(profile.totalWeeks)}
            onChange={(e) =>
              setProfile((p) => ({
                ...p,
                totalWeeks: parseIntSafe(e.currentTarget.value, p.totalWeeks, 1, 60)
              }))
            }
            style={{ display: "block", marginTop: 4, width: 120 }}
          />
        </label>
      </div>

      <label style={{ display: "block", marginTop: 12 }}>
        Strands (comma separated)
        <input
          value={profile.strands.join(", ")}
          onChange={(e) =>
            setProfile((p) => ({
              ...p,
              strands: e.currentTarget.value
                .split(",")
                .map((v) => v.trim())
                .filter((v) => v.length > 0)
            }))
          }
          style={{ display: "block", marginTop: 4, width: "100%" }}
        />
      </label>

      <label style={{ display: "block", marginTop: 12 }}>
        Policy text
        <textarea
          value={profile.policyText}
          onChange={(e) => setProfile((p) => ({ ...p, policyText: e.currentTarget.value }))}
          rows={6}
          style={{ display: "block", marginTop: 4, width: "100%" }}
        />
      </label>

      <label style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 10 }}>
        <input
          data-testid="course-description-option-include-policy"
          type="checkbox"
          checked={includePolicyDefault}
          onChange={(e) => setIncludePolicyDefault(e.currentTarget.checked)}
        />
        Include policy in generated model
      </label>
      <label style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
        <input
          data-testid="course-description-option-include-strands"
          type="checkbox"
          checked={includeStrands}
          onChange={(e) => setIncludeStrands(e.currentTarget.checked)}
        />
        Include strands in generated model
      </label>
      <label style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
        <input
          data-testid="course-description-option-include-assessment-plan"
          type="checkbox"
          checked={includeAssessmentPlan}
          onChange={(e) => setIncludeAssessmentPlan(e.currentTarget.checked)}
        />
        Include assessment plan summary
      </label>
      <label style={{ display: "flex", gap: 8, alignItems: "center", marginTop: 6 }}>
        <input
          data-testid="course-description-option-include-resources"
          type="checkbox"
          checked={includeResources}
          onChange={(e) => setIncludeResources(e.currentTarget.checked)}
        />
        Include resources list
      </label>

      <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
        <button onClick={() => void saveProfile()} disabled={saving || loading}>
          Save Profile
        </button>
        <button
          data-testid="course-description-generate-btn"
          onClick={() => void generateCourseDescription()}
          disabled={saving || loading}
        >
          Generate Course Description
        </button>
        <button
          data-testid="course-time-management-generate-btn"
          onClick={() => void generateTimeManagement()}
          disabled={saving || loading}
        >
          Generate Time Management
        </button>
      </div>

      <div style={{ marginTop: 14, display: "grid", gridTemplateColumns: "1fr 1fr", gap: 10 }}>
        <div>
          <div
            data-testid="course-description-preview-diff"
            style={{ fontSize: 12, color: "#555", marginBottom: 6 }}
          >
            {generationDiffSummary}
          </div>
          <div style={{ fontWeight: 700, marginBottom: 6 }}>Course Description Model</div>
          <pre
            style={{
              maxHeight: 300,
              overflow: "auto",
              background: "#f7f7f7",
              border: "1px solid #ddd",
              padding: 10
            }}
          >
            {generatedModel ? JSON.stringify(generatedModel, null, 2) : "(none)"}
          </pre>
        </div>

        <div>
          <div style={{ fontWeight: 700, marginBottom: 6 }}>Time Management Model</div>
          <pre
            style={{
              maxHeight: 300,
              overflow: "auto",
              background: "#f7f7f7",
              border: "1px solid #ddd",
              padding: 10
            }}
          >
            {timeManagementModel ? JSON.stringify(timeManagementModel, null, 2) : "(none)"}
          </pre>
        </div>
      </div>
    </div>
  );
}
