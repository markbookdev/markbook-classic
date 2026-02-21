import React, { useEffect, useState } from "react";
import { SetupGetResultSchema, SetupUpdateResultSchema } from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type SetupState = {
  analysis: {
    defaultStudentScope: "all" | "active" | "valid";
    showInactiveStudents: boolean;
    showDeletedEntries: boolean;
    histogramBins: number;
    defaultSortBy: "sortOrder" | "displayName" | "finalMark";
    defaultTopBottomCount: number;
  };
  marks: {
    defaultHideDeletedEntries: boolean;
    defaultAutoPreviewBeforeBulkApply: boolean;
  };
  exchange: {
    defaultExportStudentScope: "all" | "active" | "valid";
    includeStateColumnsByDefault: boolean;
  };
  analytics: {
    defaultPageSize: number;
    defaultCohortMode: "none" | "bin" | "threshold";
  };
  attendance: {
    schoolYearStartMonth: number;
    presentCode: string;
    absentCode: string;
    lateCode: string;
    excusedCode: string;
    tardyThresholdMinutes: number;
  };
  comments: {
    defaultTransferPolicy: "replace" | "append" | "fill_blank" | "source_if_longer";
    defaultSetNumber: number;
    defaultAppendSeparator: string;
    enforceMaxCharsByDefault: boolean;
    appendSeparator: string;
    enforceFit: boolean;
    enforceMaxChars: boolean;
    defaultMaxChars: number;
  };
  printer: {
    fontScale: number;
    landscapeWideTables: boolean;
    repeatHeaders: boolean;
    showGeneratedAt: boolean;
    defaultMarginMm: number;
    defaultPaperSize: "letter" | "legal" | "a4";
    defaultOrientation: "portrait" | "landscape";
  };
  integrations: {
    defaultSisProfile: "mb_exchange_v1" | "sis_roster_v1" | "sis_marks_v1";
    defaultMatchMode: "student_no_then_name" | "name_only" | "sort_order";
    defaultCollisionPolicy: "merge_existing" | "append_new" | "stop_on_collision";
    autoPreviewBeforeApply: boolean;
    adminTransferDefaultPolicy: "replace" | "append" | "fill_blank" | "source_if_longer";
  };
  planner: {
    defaultLessonDurationMinutes: number;
    defaultPublishStatus: "draft" | "published" | "archived";
    showArchivedByDefault: boolean;
    defaultUnitTitlePrefix: string;
  };
  courseDescription: {
    defaultPeriodMinutes: number;
    defaultPeriodsPerWeek: number;
    defaultTotalWeeks: number;
    includePolicyByDefault: boolean;
  };
  reports: {
    plannerHeaderStyle: "compact" | "classic" | "minimal";
    showGeneratedAt: boolean;
    defaultStudentScope: "all" | "active" | "valid";
    defaultAnalyticsScope: "all" | "active" | "valid";
    showFiltersInHeaderByDefault: boolean;
    repeatHeadersByDefault: boolean;
    defaultPageMargins: {
      topMm: number;
      rightMm: number;
      bottomMm: number;
      leftMm: number;
    };
  };
  security: {
    passwordEnabled: boolean;
    requireWorkspacePassword: boolean;
    passwordHint: string | null;
    confirmDeletes: boolean;
    autoLockMinutes: number;
  };
  email: {
    enabled: boolean;
    fromName: string;
    replyTo: string;
    subjectPrefix: string;
    defaultCc: string;
  };
};

const DEFAULT_STATE: SetupState = {
  analysis: {
    defaultStudentScope: "valid",
    showInactiveStudents: false,
    showDeletedEntries: false,
    histogramBins: 10,
    defaultSortBy: "sortOrder",
    defaultTopBottomCount: 5
  },
  marks: {
    defaultHideDeletedEntries: true,
    defaultAutoPreviewBeforeBulkApply: true
  },
  exchange: {
    defaultExportStudentScope: "valid",
    includeStateColumnsByDefault: true
  },
  analytics: {
    defaultPageSize: 25,
    defaultCohortMode: "none"
  },
  attendance: {
    schoolYearStartMonth: 9,
    presentCode: "P",
    absentCode: "A",
    lateCode: "L",
    excusedCode: "E",
    tardyThresholdMinutes: 10
  },
  comments: {
    defaultTransferPolicy: "fill_blank",
    defaultSetNumber: 1,
    defaultAppendSeparator: " ",
    enforceMaxCharsByDefault: true,
    appendSeparator: " ",
    enforceFit: true,
    enforceMaxChars: true,
    defaultMaxChars: 600
  },
  printer: {
    fontScale: 100,
    landscapeWideTables: true,
    repeatHeaders: true,
    showGeneratedAt: true,
    defaultMarginMm: 12,
    defaultPaperSize: "letter",
    defaultOrientation: "portrait"
  },
  integrations: {
    defaultSisProfile: "sis_roster_v1",
    defaultMatchMode: "student_no_then_name",
    defaultCollisionPolicy: "merge_existing",
    autoPreviewBeforeApply: true,
    adminTransferDefaultPolicy: "fill_blank"
  },
  planner: {
    defaultLessonDurationMinutes: 75,
    defaultPublishStatus: "draft",
    showArchivedByDefault: false,
    defaultUnitTitlePrefix: "Unit"
  },
  courseDescription: {
    defaultPeriodMinutes: 75,
    defaultPeriodsPerWeek: 5,
    defaultTotalWeeks: 36,
    includePolicyByDefault: true
  },
  reports: {
    plannerHeaderStyle: "classic",
    showGeneratedAt: true,
    defaultStudentScope: "valid",
    defaultAnalyticsScope: "valid",
    showFiltersInHeaderByDefault: true,
    repeatHeadersByDefault: true,
    defaultPageMargins: {
      topMm: 12,
      rightMm: 12,
      bottomMm: 12,
      leftMm: 12
    }
  },
  security: {
    passwordEnabled: false,
    requireWorkspacePassword: false,
    passwordHint: null,
    confirmDeletes: true,
    autoLockMinutes: 0
  },
  email: {
    enabled: false,
    fromName: "",
    replyTo: "",
    subjectPrefix: "MarkBook",
    defaultCc: ""
  }
};

function parseIntOr(prev: number, raw: string, min: number, max: number) {
  const n = Number(raw.trim());
  if (!Number.isFinite(n)) return prev;
  return Math.max(min, Math.min(max, Math.trunc(n)));
}

export function SetupAdminScreen(props: { onError: (msg: string | null) => void }) {
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [setup, setSetup] = useState<SetupState>(DEFAULT_STATE);

  async function load() {
    setLoading(true);
    props.onError(null);
    try {
      const res = await requestParsed("setup.get", {}, SetupGetResultSchema);
      setSetup({
        analysis: {
          defaultStudentScope: res.analysis.defaultStudentScope,
          showInactiveStudents: res.analysis.showInactiveStudents,
          showDeletedEntries: res.analysis.showDeletedEntries,
          histogramBins: res.analysis.histogramBins,
          defaultSortBy: res.analysis.defaultSortBy,
          defaultTopBottomCount: res.analysis.defaultTopBottomCount
        },
        marks: {
          defaultHideDeletedEntries: res.marks.defaultHideDeletedEntries,
          defaultAutoPreviewBeforeBulkApply: res.marks.defaultAutoPreviewBeforeBulkApply
        },
        exchange: {
          defaultExportStudentScope: res.exchange.defaultExportStudentScope,
          includeStateColumnsByDefault: res.exchange.includeStateColumnsByDefault
        },
        analytics: {
          defaultPageSize: res.analytics.defaultPageSize,
          defaultCohortMode: res.analytics.defaultCohortMode
        },
        attendance: {
          schoolYearStartMonth: res.attendance.schoolYearStartMonth,
          presentCode: res.attendance.presentCode,
          absentCode: res.attendance.absentCode,
          lateCode: res.attendance.lateCode,
          excusedCode: res.attendance.excusedCode,
          tardyThresholdMinutes: res.attendance.tardyThresholdMinutes
        },
        comments: {
          defaultTransferPolicy: res.comments.defaultTransferPolicy,
          defaultSetNumber: res.comments.defaultSetNumber,
          defaultAppendSeparator: res.comments.defaultAppendSeparator,
          enforceMaxCharsByDefault: res.comments.enforceMaxCharsByDefault,
          appendSeparator: res.comments.appendSeparator,
          enforceFit: res.comments.enforceFit,
          enforceMaxChars: res.comments.enforceMaxChars,
          defaultMaxChars: res.comments.defaultMaxChars
        },
        printer: {
          fontScale: res.printer.fontScale,
          landscapeWideTables: res.printer.landscapeWideTables,
          repeatHeaders: res.printer.repeatHeaders,
          showGeneratedAt: res.printer.showGeneratedAt,
          defaultMarginMm: res.printer.defaultMarginMm,
          defaultPaperSize: res.printer.defaultPaperSize,
          defaultOrientation: res.printer.defaultOrientation
        },
        integrations: {
          defaultSisProfile: res.integrations.defaultSisProfile,
          defaultMatchMode: res.integrations.defaultMatchMode,
          defaultCollisionPolicy: res.integrations.defaultCollisionPolicy,
          autoPreviewBeforeApply: res.integrations.autoPreviewBeforeApply,
          adminTransferDefaultPolicy: res.integrations.adminTransferDefaultPolicy
        },
        planner: {
          defaultLessonDurationMinutes: res.planner.defaultLessonDurationMinutes,
          defaultPublishStatus: res.planner.defaultPublishStatus,
          showArchivedByDefault: res.planner.showArchivedByDefault,
          defaultUnitTitlePrefix: res.planner.defaultUnitTitlePrefix
        },
        courseDescription: {
          defaultPeriodMinutes: res.courseDescription.defaultPeriodMinutes,
          defaultPeriodsPerWeek: res.courseDescription.defaultPeriodsPerWeek,
          defaultTotalWeeks: res.courseDescription.defaultTotalWeeks,
          includePolicyByDefault: res.courseDescription.includePolicyByDefault
        },
        reports: {
          plannerHeaderStyle: res.reports.plannerHeaderStyle,
          showGeneratedAt: res.reports.showGeneratedAt,
          defaultStudentScope: res.reports.defaultStudentScope,
          defaultAnalyticsScope: res.reports.defaultAnalyticsScope,
          showFiltersInHeaderByDefault: res.reports.showFiltersInHeaderByDefault,
          repeatHeadersByDefault: res.reports.repeatHeadersByDefault,
          defaultPageMargins: res.reports.defaultPageMargins
        },
        security: {
          passwordEnabled: res.security.passwordEnabled,
          requireWorkspacePassword: res.security.requireWorkspacePassword,
          passwordHint: res.security.passwordHint,
          confirmDeletes: res.security.confirmDeletes,
          autoLockMinutes: res.security.autoLockMinutes
        },
        email: {
          enabled: res.email.enabled,
          fromName: res.email.fromName,
          replyTo: res.email.replyTo,
          subjectPrefix: res.email.subjectPrefix,
          defaultCc: res.email.defaultCc
        }
      });
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void load();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function saveSection(section: keyof SetupState) {
    setSaving(true);
    props.onError(null);
    try {
      await requestParsed(
        "setup.update",
        {
          section,
          patch: setup[section]
        },
        SetupUpdateResultSchema
      );
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  async function saveAll() {
    setSaving(true);
    props.onError(null);
    try {
      for (const section of [
        "analysis",
        "marks",
        "exchange",
        "analytics",
        "attendance",
        "comments",
        "printer",
        "integrations",
        "planner",
        "courseDescription",
        "reports",
        "security",
        "email"
      ] as const) {
        await requestParsed(
          "setup.update",
          {
            section,
            patch: setup[section]
          },
          SetupUpdateResultSchema
        );
      }
    } catch (e: any) {
      props.onError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div data-testid="setup-admin-screen" style={{ padding: 24, maxWidth: 1100 }}>
      <div style={{ fontWeight: 800, fontSize: 22, marginBottom: 10 }}>Setup & Admin</div>
      <div style={{ color: "#666", marginBottom: 14 }}>
        Workspace-level defaults for analytics, attendance, comments transfer, printing, security,
        integrations, and email metadata.
      </div>

      <div style={{ display: "flex", gap: 8, marginBottom: 14 }}>
        <button data-testid="setup-save-all" onClick={() => void saveAll()} disabled={saving || loading}>
          Save All
        </button>
        <button data-testid="setup-reload" onClick={() => void load()} disabled={saving || loading}>
          Reload
        </button>
      </div>

      <div
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(360px, 1fr))",
          gap: 12
        }}
      >
        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Analysis Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default Student Scope
            <select
              data-testid="setup-analysis-scope"
              value={setup.analysis.defaultStudentScope}
              onChange={(e) =>
                {
                  const value =
                    e.currentTarget.value as SetupState["analysis"]["defaultStudentScope"];
                  setSetup((s) => ({
                    ...s,
                    analysis: {
                      ...s.analysis,
                      defaultStudentScope: value
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="all">all</option>
              <option value="active">active</option>
              <option value="valid">valid</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.analysis.showInactiveStudents}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    analysis: { ...s.analysis, showInactiveStudents: checked }
                  }));
                }
              }
            />
            Show inactive students by default
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.analysis.showDeletedEntries}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    analysis: { ...s.analysis, showDeletedEntries: checked }
                  }));
                }
              }
            />
            Show deleted entries by default
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Histogram bins
            <input
              value={String(setup.analysis.histogramBins)}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    analysis: {
                      ...s.analysis,
                      histogramBins: parseIntOr(s.analysis.histogramBins, value, 4, 20)
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default sort by
            <select
              value={setup.analysis.defaultSortBy}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["analysis"]["defaultSortBy"];
                setSetup((s) => ({
                  ...s,
                  analysis: { ...s.analysis, defaultSortBy: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="sortOrder">sortOrder</option>
              <option value="displayName">displayName</option>
              <option value="finalMark">finalMark</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default top/bottom count
            <input
              value={String(setup.analysis.defaultTopBottomCount)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  analysis: {
                    ...s.analysis,
                    defaultTopBottomCount: parseIntOr(
                      s.analysis.defaultTopBottomCount,
                      e.currentTarget.value,
                      3,
                      20
                    )
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <button data-testid="setup-save-analysis" onClick={() => void saveSection("analysis")} disabled={saving || loading}>
            Save Analysis
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Marks Defaults</div>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-marks-default-hide-deleted"
              type="checkbox"
              checked={setup.marks.defaultHideDeletedEntries}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  marks: { ...s.marks, defaultHideDeletedEntries: checked }
                }));
              }}
            />
            Hide deleted entries by default
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-marks-auto-preview-bulk"
              type="checkbox"
              checked={setup.marks.defaultAutoPreviewBeforeBulkApply}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  marks: { ...s.marks, defaultAutoPreviewBeforeBulkApply: checked }
                }));
              }}
            />
            Confirm before applying bulk row updates
          </label>
          <button
            data-testid="setup-save-marks"
            onClick={() => void saveSection("marks")}
            disabled={saving || loading}
          >
            Save Marks
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Exchange Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default export student scope
            <select
              data-testid="setup-exchange-default-scope"
              value={setup.exchange.defaultExportStudentScope}
              onChange={(e) => {
                const value =
                  e.currentTarget.value as SetupState["exchange"]["defaultExportStudentScope"];
                setSetup((s) => ({
                  ...s,
                  exchange: { ...s.exchange, defaultExportStudentScope: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="all">all</option>
              <option value="active">active</option>
              <option value="valid">valid</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-exchange-include-state-columns"
              type="checkbox"
              checked={setup.exchange.includeStateColumnsByDefault}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  exchange: { ...s.exchange, includeStateColumnsByDefault: checked }
                }));
              }}
            />
            Include mark state columns by default in SIS marks export
          </label>
          <button
            data-testid="setup-save-exchange"
            onClick={() => void saveSection("exchange")}
            disabled={saving || loading}
          >
            Save Exchange
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Analytics Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default page size
            <input
              data-testid="setup-analytics-default-page-size"
              value={String(setup.analytics.defaultPageSize)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  analytics: {
                    ...s.analytics,
                    defaultPageSize: parseIntOr(
                      s.analytics.defaultPageSize,
                      e.currentTarget.value,
                      10,
                      200
                    )
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default cohort mode
            <select
              data-testid="setup-analytics-default-cohort-mode"
              value={setup.analytics.defaultCohortMode}
              onChange={(e) => {
                const value =
                  e.currentTarget.value as SetupState["analytics"]["defaultCohortMode"];
                setSetup((s) => ({
                  ...s,
                  analytics: { ...s.analytics, defaultCohortMode: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="none">none</option>
              <option value="bin">bin</option>
              <option value="threshold">threshold</option>
            </select>
          </label>
          <button
            data-testid="setup-save-analytics"
            onClick={() => void saveSection("analytics")}
            disabled={saving || loading}
          >
            Save Analytics
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Attendance Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            School year start month
            <input
              value={String(setup.attendance.schoolYearStartMonth)}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    attendance: {
                      ...s.attendance,
                      schoolYearStartMonth: parseIntOr(
                        s.attendance.schoolYearStartMonth,
                        value,
                        1,
                        12
                      )
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8 }}>
            <label>
              Present code
              <input
                value={setup.attendance.presentCode}
                onChange={(e) =>
                  {
                    const value = e.currentTarget.value.toUpperCase();
                    setSetup((s) => ({
                      ...s,
                      attendance: { ...s.attendance, presentCode: value }
                    }));
                  }
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Absent code
              <input
                value={setup.attendance.absentCode}
                onChange={(e) =>
                  {
                    const value = e.currentTarget.value.toUpperCase();
                    setSetup((s) => ({
                      ...s,
                      attendance: { ...s.attendance, absentCode: value }
                    }));
                  }
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Late code
              <input
                value={setup.attendance.lateCode}
                onChange={(e) =>
                  {
                    const value = e.currentTarget.value.toUpperCase();
                    setSetup((s) => ({
                      ...s,
                      attendance: { ...s.attendance, lateCode: value }
                    }));
                  }
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Excused code
              <input
                value={setup.attendance.excusedCode}
                onChange={(e) =>
                  {
                    const value = e.currentTarget.value.toUpperCase();
                    setSetup((s) => ({
                      ...s,
                      attendance: { ...s.attendance, excusedCode: value }
                    }));
                  }
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
          </div>
          <label style={{ display: "block", marginTop: 8, marginBottom: 8 }}>
            Tardy threshold (minutes)
            <input
              value={String(setup.attendance.tardyThresholdMinutes)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  attendance: {
                    ...s.attendance,
                    tardyThresholdMinutes: parseIntOr(
                      s.attendance.tardyThresholdMinutes,
                      e.currentTarget.value,
                      0,
                      120
                    )
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <div style={{ marginTop: 8 }}>
            <button data-testid="setup-save-attendance" onClick={() => void saveSection("attendance")} disabled={saving || loading}>
              Save Attendance
            </button>
          </div>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Comments Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default transfer policy
            <select
              data-testid="setup-comments-policy"
              value={setup.comments.defaultTransferPolicy}
              onChange={(e) =>
                {
                  const value =
                    e.currentTarget.value as SetupState["comments"]["defaultTransferPolicy"];
                  setSetup((s) => ({
                    ...s,
                    comments: {
                      ...s.comments,
                      defaultTransferPolicy: value
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="fill_blank">fill_blank</option>
              <option value="replace">replace</option>
              <option value="append">append</option>
              <option value="source_if_longer">source_if_longer</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default set number
            <input
              data-testid="setup-comments-default-set-number"
              value={String(setup.comments.defaultSetNumber)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  comments: {
                    ...s.comments,
                    defaultSetNumber: parseIntOr(s.comments.defaultSetNumber, e.currentTarget.value, 1, 20)
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Append separator
            <input
              data-testid="setup-comments-append-separator"
              value={setup.comments.defaultAppendSeparator}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    comments: {
                      ...s.comments,
                      defaultAppendSeparator: value,
                      appendSeparator: value
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.comments.enforceFit}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    comments: { ...s.comments, enforceFit: checked }
                  }));
                }
              }
            />
            Enforce fit constraints
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-comments-enforce-max-default"
              type="checkbox"
              checked={setup.comments.enforceMaxCharsByDefault}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    comments: {
                      ...s.comments,
                      enforceMaxCharsByDefault: checked,
                      enforceMaxChars: checked
                    }
                  }));
                }
              }
            />
            Enforce max chars
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default max chars
            <input
              value={String(setup.comments.defaultMaxChars)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  comments: {
                    ...s.comments,
                    defaultMaxChars: parseIntOr(s.comments.defaultMaxChars, e.currentTarget.value, 80, 5000)
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <button data-testid="setup-save-comments" onClick={() => void saveSection("comments")} disabled={saving || loading}>
            Save Comments
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Printer Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Font scale (%)
            <input
              data-testid="setup-printer-font-scale"
              value={String(setup.printer.fontScale)}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    printer: {
                      ...s.printer,
                      fontScale: parseIntOr(s.printer.fontScale, value, 60, 160)
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.printer.landscapeWideTables}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    printer: { ...s.printer, landscapeWideTables: checked }
                  }));
                }
              }
            />
            Landscape for wide tables
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.printer.repeatHeaders}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    printer: { ...s.printer, repeatHeaders: checked }
                  }));
                }
              }
            />
            Repeat table headers
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.printer.showGeneratedAt}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    printer: { ...s.printer, showGeneratedAt: checked }
                  }));
                }
              }
            />
            Include generated timestamp
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default margin (mm)
            <input
              value={String(setup.printer.defaultMarginMm)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  printer: {
                    ...s.printer,
                    defaultMarginMm: parseIntOr(s.printer.defaultMarginMm, e.currentTarget.value, 5, 30)
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 100 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default paper size
            <select
              data-testid="setup-printer-paper-size"
              value={setup.printer.defaultPaperSize}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["printer"]["defaultPaperSize"];
                setSetup((s) => ({
                  ...s,
                  printer: { ...s.printer, defaultPaperSize: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="letter">letter</option>
              <option value="legal">legal</option>
              <option value="a4">a4</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default orientation
            <select
              data-testid="setup-printer-orientation"
              value={setup.printer.defaultOrientation}
              onChange={(e) => {
                const value =
                  e.currentTarget.value as SetupState["printer"]["defaultOrientation"];
                setSetup((s) => ({
                  ...s,
                  printer: { ...s.printer, defaultOrientation: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="portrait">portrait</option>
              <option value="landscape">landscape</option>
            </select>
          </label>
          <button data-testid="setup-save-printer" onClick={() => void saveSection("printer")} disabled={saving || loading}>
            Save Printer
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Security Defaults</div>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-security-require-workspace-password"
              type="checkbox"
              checked={setup.security.requireWorkspacePassword}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    security: {
                      ...s.security,
                      requireWorkspacePassword: checked,
                      passwordEnabled: checked
                    }
                  }));
                }
              }
            />
            Require workspace password
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Password hint (optional)
            <input
              value={setup.security.passwordHint ?? ""}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    security: {
                      ...s.security,
                      passwordHint: value.trim().length ? value : null
                    }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: "100%" }}
            />
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.security.confirmDeletes}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    security: { ...s.security, confirmDeletes: checked }
                  }));
                }
              }
            />
            Confirm destructive actions
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Auto-lock after inactivity (minutes)
            <input
              value={String(setup.security.autoLockMinutes)}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  security: {
                    ...s.security,
                    autoLockMinutes: parseIntOr(s.security.autoLockMinutes, e.currentTarget.value, 0, 240)
                  }
                }))
              }
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <button data-testid="setup-save-security" onClick={() => void saveSection("security")} disabled={saving || loading}>
            Save Security
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Integrations Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default SIS profile
            <select
              data-testid="setup-integrations-default-profile"
              value={setup.integrations.defaultSisProfile}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["integrations"]["defaultSisProfile"];
                setSetup((s) => ({
                  ...s,
                  integrations: { ...s.integrations, defaultSisProfile: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="sis_roster_v1">sis_roster_v1</option>
              <option value="sis_marks_v1">sis_marks_v1</option>
              <option value="mb_exchange_v1">mb_exchange_v1</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default match mode
            <select
              data-testid="setup-integrations-match-mode"
              value={setup.integrations.defaultMatchMode}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["integrations"]["defaultMatchMode"];
                setSetup((s) => ({
                  ...s,
                  integrations: { ...s.integrations, defaultMatchMode: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="student_no_then_name">student_no_then_name</option>
              <option value="name_only">name_only</option>
              <option value="sort_order">sort_order</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default collision policy
            <select
              data-testid="setup-integrations-collision-policy"
              value={setup.integrations.defaultCollisionPolicy}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["integrations"]["defaultCollisionPolicy"];
                setSetup((s) => ({
                  ...s,
                  integrations: { ...s.integrations, defaultCollisionPolicy: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="merge_existing">merge_existing</option>
              <option value="append_new">append_new</option>
              <option value="stop_on_collision">stop_on_collision</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.integrations.autoPreviewBeforeApply}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  integrations: { ...s.integrations, autoPreviewBeforeApply: checked }
                }));
              }}
            />
            Require preview before apply
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Admin transfer comments policy
            <select
              data-testid="setup-integrations-admin-policy"
              value={setup.integrations.adminTransferDefaultPolicy}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["integrations"]["adminTransferDefaultPolicy"];
                setSetup((s) => ({
                  ...s,
                  integrations: { ...s.integrations, adminTransferDefaultPolicy: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="fill_blank">fill_blank</option>
              <option value="replace">replace</option>
              <option value="append">append</option>
              <option value="source_if_longer">source_if_longer</option>
            </select>
          </label>
          <button
            data-testid="setup-save-integrations"
            onClick={() => void saveSection("integrations")}
            disabled={saving || loading}
          >
            Save Integrations
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Planner Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default lesson duration (minutes)
            <input
              data-testid="setup-planner-duration"
              value={String(setup.planner.defaultLessonDurationMinutes)}
              onChange={(e) => {
                const value = e.currentTarget.value;
                setSetup((s) => ({
                  ...s,
                  planner: {
                    ...s.planner,
                    defaultLessonDurationMinutes: parseIntOr(
                      s.planner.defaultLessonDurationMinutes,
                      value,
                      15,
                      240
                    )
                  }
                }));
              }}
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default publish status
            <select
              data-testid="setup-planner-publish-status"
              value={setup.planner.defaultPublishStatus}
              onChange={(e) => {
                const value =
                  e.currentTarget.value as SetupState["planner"]["defaultPublishStatus"];
                setSetup((s) => ({
                  ...s,
                  planner: { ...s.planner, defaultPublishStatus: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="draft">draft</option>
              <option value="published">published</option>
              <option value="archived">archived</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.planner.showArchivedByDefault}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  planner: { ...s.planner, showArchivedByDefault: checked }
                }));
              }}
            />
            Show archived records by default
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Unit title prefix
            <input
              value={setup.planner.defaultUnitTitlePrefix}
              onChange={(e) => {
                const value = e.currentTarget.value;
                setSetup((s) => ({
                  ...s,
                  planner: { ...s.planner, defaultUnitTitlePrefix: value }
                }));
              }}
              style={{ display: "block", marginTop: 4, width: 160 }}
            />
          </label>
          <button
            data-testid="setup-save-planner"
            onClick={() => void saveSection("planner")}
            disabled={saving || loading}
          >
            Save Planner
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Course Description Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default period minutes
            <input
              value={String(setup.courseDescription.defaultPeriodMinutes)}
              onChange={(e) => {
                const value = e.currentTarget.value;
                setSetup((s) => ({
                  ...s,
                  courseDescription: {
                    ...s.courseDescription,
                    defaultPeriodMinutes: parseIntOr(
                      s.courseDescription.defaultPeriodMinutes,
                      value,
                      1,
                      300
                    )
                  }
                }));
              }}
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default periods per week
            <input
              data-testid="setup-course-periods"
              value={String(setup.courseDescription.defaultPeriodsPerWeek)}
              onChange={(e) => {
                const value = e.currentTarget.value;
                setSetup((s) => ({
                  ...s,
                  courseDescription: {
                    ...s.courseDescription,
                    defaultPeriodsPerWeek: parseIntOr(
                      s.courseDescription.defaultPeriodsPerWeek,
                      value,
                      1,
                      14
                    )
                  }
                }));
              }}
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default total weeks
            <input
              data-testid="setup-course-weeks"
              value={String(setup.courseDescription.defaultTotalWeeks)}
              onChange={(e) => {
                const value = e.currentTarget.value;
                setSetup((s) => ({
                  ...s,
                  courseDescription: {
                    ...s.courseDescription,
                    defaultTotalWeeks: parseIntOr(
                      s.courseDescription.defaultTotalWeeks,
                      value,
                      1,
                      60
                    )
                  }
                }));
              }}
              style={{ display: "block", marginTop: 4, width: 120 }}
            />
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.courseDescription.includePolicyByDefault}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  courseDescription: {
                    ...s.courseDescription,
                    includePolicyByDefault: checked
                  }
                }));
              }}
            />
            Include policy text by default
          </label>
          <button
            data-testid="setup-save-course-description"
            onClick={() => void saveSection("courseDescription")}
            disabled={saving || loading}
          >
            Save Course Description
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Report Defaults</div>
          <label style={{ display: "block", marginBottom: 8 }}>
            Planner header style
            <select
              data-testid="setup-reports-planner-header-style"
              value={setup.reports.plannerHeaderStyle}
              onChange={(e) => {
                const value =
                  e.currentTarget.value as SetupState["reports"]["plannerHeaderStyle"];
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, plannerHeaderStyle: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="classic">classic</option>
              <option value="compact">compact</option>
              <option value="minimal">minimal</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.reports.showGeneratedAt}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, showGeneratedAt: checked }
                }));
              }}
            />
            Include generated timestamp on reports
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default report student scope
            <select
              value={setup.reports.defaultStudentScope}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["reports"]["defaultStudentScope"];
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, defaultStudentScope: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="all">all</option>
              <option value="active">active</option>
              <option value="valid">valid</option>
            </select>
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default analytics scope
            <select
              data-testid="setup-reports-default-analytics-scope"
              value={setup.reports.defaultAnalyticsScope}
              onChange={(e) => {
                const value = e.currentTarget.value as SetupState["reports"]["defaultAnalyticsScope"];
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, defaultAnalyticsScope: value }
                }));
              }}
              style={{ display: "block", marginTop: 4 }}
            >
              <option value="all">all</option>
              <option value="active">active</option>
              <option value="valid">valid</option>
            </select>
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.reports.showFiltersInHeaderByDefault}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, showFiltersInHeaderByDefault: checked }
                }));
              }}
            />
            Show filter metadata in report headers
          </label>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              data-testid="setup-reports-repeat-headers"
              type="checkbox"
              checked={setup.reports.repeatHeadersByDefault}
              onChange={(e) => {
                const checked = e.currentTarget.checked;
                setSetup((s) => ({
                  ...s,
                  reports: { ...s.reports, repeatHeadersByDefault: checked }
                }));
              }}
            />
            Repeat table headers by default
          </label>
          <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr", gap: 8, marginBottom: 8 }}>
            <label>
              Margin top (mm)
              <input
                data-testid="setup-reports-margin-top"
                value={String(setup.reports.defaultPageMargins.topMm)}
                onChange={(e) =>
                  setSetup((s) => ({
                    ...s,
                    reports: {
                      ...s.reports,
                      defaultPageMargins: {
                        ...s.reports.defaultPageMargins,
                        topMm: parseIntOr(s.reports.defaultPageMargins.topMm, e.currentTarget.value, 0, 40)
                      }
                    }
                  }))
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Margin right (mm)
              <input
                data-testid="setup-reports-margin-right"
                value={String(setup.reports.defaultPageMargins.rightMm)}
                onChange={(e) =>
                  setSetup((s) => ({
                    ...s,
                    reports: {
                      ...s.reports,
                      defaultPageMargins: {
                        ...s.reports.defaultPageMargins,
                        rightMm: parseIntOr(s.reports.defaultPageMargins.rightMm, e.currentTarget.value, 0, 40)
                      }
                    }
                  }))
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Margin bottom (mm)
              <input
                data-testid="setup-reports-margin-bottom"
                value={String(setup.reports.defaultPageMargins.bottomMm)}
                onChange={(e) =>
                  setSetup((s) => ({
                    ...s,
                    reports: {
                      ...s.reports,
                      defaultPageMargins: {
                        ...s.reports.defaultPageMargins,
                        bottomMm: parseIntOr(s.reports.defaultPageMargins.bottomMm, e.currentTarget.value, 0, 40)
                      }
                    }
                  }))
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
            <label>
              Margin left (mm)
              <input
                data-testid="setup-reports-margin-left"
                value={String(setup.reports.defaultPageMargins.leftMm)}
                onChange={(e) =>
                  setSetup((s) => ({
                    ...s,
                    reports: {
                      ...s.reports,
                      defaultPageMargins: {
                        ...s.reports.defaultPageMargins,
                        leftMm: parseIntOr(s.reports.defaultPageMargins.leftMm, e.currentTarget.value, 0, 40)
                      }
                    }
                  }))
                }
                style={{ display: "block", marginTop: 4, width: "100%" }}
              />
            </label>
          </div>
          <button
            data-testid="setup-save-reports"
            onClick={() => void saveSection("reports")}
            disabled={saving || loading}
          >
            Save Reports
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Email Metadata</div>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.email.enabled}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    email: { ...s.email, enabled: checked }
                  }));
                }
              }
            />
            Enable email metadata in workflow
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            From name
            <input
              value={setup.email.fromName}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    email: { ...s.email, fromName: value }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: "100%" }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Reply-to
            <input
              value={setup.email.replyTo}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    email: { ...s.email, replyTo: value }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: "100%" }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Subject prefix
            <input
              value={setup.email.subjectPrefix}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    email: { ...s.email, subjectPrefix: value }
                  }));
                }
              }
              style={{ display: "block", marginTop: 4, width: "100%" }}
            />
          </label>
          <label style={{ display: "block", marginBottom: 8 }}>
            Default CC
            <input
              value={setup.email.defaultCc}
              onChange={(e) =>
                setSetup((s) => ({
                  ...s,
                  email: { ...s.email, defaultCc: e.currentTarget.value }
                }))
              }
              style={{ display: "block", marginTop: 4, width: "100%" }}
            />
          </label>
          <button data-testid="setup-save-email" onClick={() => void saveSection("email")} disabled={saving || loading}>
            Save Email
          </button>
        </section>
      </div>
    </div>
  );
}
