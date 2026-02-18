const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("learning skills edits persist", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    const first = await page.evaluate(async ({ classId }) => {
      const open = await window.markbook.request("learningSkills.open", { classId, term: 1 });
      return {
        studentId: open.students?.[0]?.id,
        skillCode: open.skillCodes?.[0] ?? "R",
      };
    }, { classId });

    expect(first.studentId).toBeTruthy();
    await page.evaluate(async ({ classId, studentId, skillCode }) => {
      await window.markbook.request("learningSkills.updateCell", {
        classId,
        studentId,
        term: 1,
        skillCode,
        value: "E",
      });
    }, { classId, studentId: first.studentId, skillCode: first.skillCode });

    const persisted = await page.evaluate(async ({ classId, studentId, skillCode }) => {
      const open = await window.markbook.request("learningSkills.open", { classId, term: 1 });
      const row = open.rows.find((r) => r.studentId === studentId);
      return row?.values?.[skillCode] ?? "";
    }, { classId, studentId: first.studentId, skillCode: first.skillCode });

    expect(persisted).toBe("E");
  } finally {
    await app.close();
  }
});
