const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("learning skills summary report exports PDF", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );

    await page.evaluate(async ({ classId }) => {
      const open = await window.markbook.request("learningSkills.open", { classId, term: 1 });
      const studentId = open.students?.[0]?.id;
      const skillCode = open.skillCodes?.[0] ?? "R";
      if (studentId) {
        await window.markbook.request("learningSkills.updateCell", {
          classId,
          studentId,
          term: 1,
          skillCode,
          value: "G",
        });
      }
    }, { classId });

    const outDir = mkdtemp("markbook-pdf-ls-");
    const outPath = path.join(outDir, "learning-skills.pdf");
    await page.evaluate(async ({ classId, outPath }) => {
      return await window.__markbookTest.exportLearningSkillsSummaryPdfToPath(
        classId,
        1,
        outPath
      );
    }, { classId, outPath });

    expect(fs.existsSync(outPath)).toBeTruthy();
    expect(fs.statSync(outPath).size).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
