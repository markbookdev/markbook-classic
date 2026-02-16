const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("attendance monthly report exports PDF", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    const outDir = mkdtemp("markbook-pdf-attendance-");
    const outPath = path.join(outDir, "attendance.pdf");

    await page.evaluate(async ({ classId, outPath }) => {
      return await window.__markbookTest.exportAttendanceMonthlyPdfToPath(
        classId,
        "9",
        outPath
      );
    }, { classId, outPath });

    expect(fs.existsSync(outPath)).toBeTruthy();
    expect(fs.statSync(outPath).size).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
