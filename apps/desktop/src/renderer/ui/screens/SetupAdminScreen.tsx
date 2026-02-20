import React, { useEffect, useState } from "react";
import { SetupGetResultSchema, SetupUpdateResultSchema } from "@markbook/schema";
import { requestParsed } from "../state/workspace";

type SetupState = {
  analysis: {
    defaultStudentScope: "all" | "active" | "valid";
    showInactiveStudents: boolean;
    showDeletedEntries: boolean;
    histogramBins: number;
  };
  attendance: {
    schoolYearStartMonth: number;
    presentCode: string;
    absentCode: string;
    lateCode: string;
    excusedCode: string;
  };
  comments: {
    defaultTransferPolicy: "replace" | "append" | "fill_blank" | "source_if_longer";
    appendSeparator: string;
    enforceFit: boolean;
    enforceMaxChars: boolean;
  };
  printer: {
    fontScale: number;
    landscapeWideTables: boolean;
    repeatHeaders: boolean;
    showGeneratedAt: boolean;
  };
  security: {
    passwordEnabled: boolean;
    passwordHint: string | null;
    confirmDeletes: boolean;
  };
  email: {
    enabled: boolean;
    fromName: string;
    replyTo: string;
    subjectPrefix: string;
  };
};

const DEFAULT_STATE: SetupState = {
  analysis: {
    defaultStudentScope: "valid",
    showInactiveStudents: false,
    showDeletedEntries: false,
    histogramBins: 10
  },
  attendance: {
    schoolYearStartMonth: 9,
    presentCode: "P",
    absentCode: "A",
    lateCode: "L",
    excusedCode: "E"
  },
  comments: {
    defaultTransferPolicy: "fill_blank",
    appendSeparator: " ",
    enforceFit: true,
    enforceMaxChars: true
  },
  printer: {
    fontScale: 100,
    landscapeWideTables: true,
    repeatHeaders: true,
    showGeneratedAt: true
  },
  security: {
    passwordEnabled: false,
    passwordHint: null,
    confirmDeletes: true
  },
  email: {
    enabled: false,
    fromName: "",
    replyTo: "",
    subjectPrefix: "MarkBook"
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
          histogramBins: res.analysis.histogramBins
        },
        attendance: {
          schoolYearStartMonth: res.attendance.schoolYearStartMonth,
          presentCode: res.attendance.presentCode,
          absentCode: res.attendance.absentCode,
          lateCode: res.attendance.lateCode,
          excusedCode: res.attendance.excusedCode
        },
        comments: {
          defaultTransferPolicy: res.comments.defaultTransferPolicy,
          appendSeparator: res.comments.appendSeparator,
          enforceFit: res.comments.enforceFit,
          enforceMaxChars: res.comments.enforceMaxChars
        },
        printer: {
          fontScale: res.printer.fontScale,
          landscapeWideTables: res.printer.landscapeWideTables,
          repeatHeaders: res.printer.repeatHeaders,
          showGeneratedAt: res.printer.showGeneratedAt
        },
        security: {
          passwordEnabled: res.security.passwordEnabled,
          passwordHint: res.security.passwordHint,
          confirmDeletes: res.security.confirmDeletes
        },
        email: {
          enabled: res.email.enabled,
          fromName: res.email.fromName,
          replyTo: res.email.replyTo,
          subjectPrefix: res.email.subjectPrefix
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
        "attendance",
        "comments",
        "printer",
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
        and email metadata.
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
          <button data-testid="setup-save-analysis" onClick={() => void saveSection("analysis")} disabled={saving || loading}>
            Save Analysis
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
            Append separator
            <input
              value={setup.comments.appendSeparator}
              onChange={(e) =>
                {
                  const value = e.currentTarget.value;
                  setSetup((s) => ({
                    ...s,
                    comments: { ...s.comments, appendSeparator: value }
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
              type="checkbox"
              checked={setup.comments.enforceMaxChars}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    comments: { ...s.comments, enforceMaxChars: checked }
                  }));
                }
              }
            />
            Enforce max chars
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
          <button data-testid="setup-save-printer" onClick={() => void saveSection("printer")} disabled={saving || loading}>
            Save Printer
          </button>
        </section>

        <section style={{ border: "1px solid #ddd", borderRadius: 10, padding: 12 }}>
          <div style={{ fontWeight: 700, marginBottom: 8 }}>Security Defaults</div>
          <label style={{ display: "flex", gap: 8, marginBottom: 8, alignItems: "center" }}>
            <input
              type="checkbox"
              checked={setup.security.passwordEnabled}
              onChange={(e) =>
                {
                  const checked = e.currentTarget.checked;
                  setSetup((s) => ({
                    ...s,
                    security: { ...s.security, passwordEnabled: checked }
                  }));
                }
              }
            />
            Enable password prompt
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
          <button data-testid="setup-save-security" onClick={() => void saveSection("security")} disabled={saving || loading}>
            Save Security
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
          <button data-testid="setup-save-email" onClick={() => void saveSection("email")} disabled={saving || loading}>
            Save Email
          </button>
        </section>
      </div>
    </div>
  );
}
