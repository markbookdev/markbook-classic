const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("student summary report respects mark set membership (valid_kid)", async () => {
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const appDir = path.join(repoRoot, "apps", "desktop");
  const legacyClassFolderPath = path.join(repoRoot, "fixtures", "legacy", "Sample25", "MB8D25");

  const userDataDir = mkdtemp("markbook-userdata-");
  const workspacePath = mkdtemp("markbook-workspace-");
  const outDir = mkdtemp("markbook-out-");
  const outPath = path.join(outDir, "student-summary.pdf");

  const electronExecutable = require("electron");
  const mainPath = path.join(appDir, "electron", "main.js");

  const app = await electron.launch({
    executablePath: electronExecutable,
    args: [mainPath],
    env: {
      ...process.env,
      VITE_DEV_SERVER_URL: "",
      MARKBOOK_USER_DATA_DIR: userDataDir,
    },
  });

  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForSelector('[data-testid="app-shell"]');

  const bootstrap = await page.evaluate(async (payload) => {
    const { workspacePath, legacyClassFolderPath } = payload;
    await window.markbook.request("workspace.select", { path: workspacePath });
    await window.markbook.request("class.importLegacy", { legacyClassFolderPath });
    const cls = await window.markbook.request("classes.list", {});
    const classId = cls.classes[0].id;
    const ms = await window.markbook.request("marksets.list", { classId });
    const mat1 = ms.markSets.find((m) => m.code === "MAT1") ?? ms.markSets[0];
    const markSetId = mat1.id;

    const sum = await window.markbook.request("calc.markSetSummary", { classId, markSetId });
    const row = sum.perStudent.find((s) => s.finalMark != null);
    if (!row) throw new Error("expected at least one student with a final mark");
    const studentId = row.studentId;

    await window.markbook.request("students.membership.set", {
      classId,
      studentId,
      markSetId,
      enabled: false,
    });

    const model = await window.markbook.request("reports.studentSummaryModel", {
      classId,
      markSetId,
      studentId,
    });
    return { classId, markSetId, studentId, modelFinal: model.student.finalMark ?? null };
  }, { workspacePath, legacyClassFolderPath });

  expect(bootstrap.modelFinal).toBeNull();

  // Export PDF via renderer report pipeline.
  await page.evaluate(async ({ classId, markSetId, studentId, outPath }) => {
    await window.__markbookTest.exportStudentSummaryPdfToPath(classId, markSetId, studentId, outPath);
  }, { ...bootstrap, outPath });

  const st = fs.statSync(outPath);
  expect(st.size).toBeGreaterThan(1000);

  await app.close();
});

