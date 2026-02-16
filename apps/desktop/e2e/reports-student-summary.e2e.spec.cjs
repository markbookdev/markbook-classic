const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const fs = require("node:fs");
const path = require("node:path");

test("export student summary PDF writes a non-empty file", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const studentId = await page.evaluate(async ({ classId, markSetId }) => {
      const open = await window.markbook.request("markset.open", { classId, markSetId });
      return open.students[0]?.id ?? null;
    }, { classId, markSetId });
    expect(studentId).toBeTruthy();

    const outDir = mkdtemp("markbook-pdf-student-");
    const outPath = path.join(outDir, "student-summary.pdf");

    await page.evaluate(async ({ classId, markSetId, studentId, outPath }) => {
      if (!window.__markbookTest?.exportStudentSummaryPdfToPath) {
        throw new Error("missing window.__markbookTest.exportStudentSummaryPdfToPath");
      }
      await window.__markbookTest.exportStudentSummaryPdfToPath(
        classId,
        markSetId,
        studentId,
        outPath
      );
    }, { classId, markSetId, studentId, outPath });

    for (let i = 0; i < 50; i += 1) {
      if (fs.existsSync(outPath) && fs.statSync(outPath).size > 0) break;
      // eslint-disable-next-line no-await-in-loop
      await new Promise((r) => setTimeout(r, 100));
    }

    expect(fs.existsSync(outPath)).toBeTruthy();
    expect(fs.statSync(outPath).size).toBeGreaterThan(1000);
  } finally {
    await app.close();
  }
});

