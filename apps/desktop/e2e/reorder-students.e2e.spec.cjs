const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("student reorder updates marks row mapping (UI -> persisted)", async () => {
  const { app, page, repoRoot } = await launchElectronApp();

  try {
    const { workspacePath, classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    // Choose a target student by current row index, set a unique score, then move them to the top.
    const { studentId } = await page.evaluate(async ({ classId, markSetId }) => {
      const open = await window.markbook.request("markset.open", { classId, markSetId });
      const studentId = open.students[5]?.id;
      if (!studentId) throw new Error("not enough students in fixture");

      await window.markbook.request("grid.updateCell", {
        classId,
        markSetId,
        row: 5,
        col: 0,
        value: 9.9,
        editKind: "set",
      });

      const g = await window.markbook.request("grid.get", {
        classId,
        markSetId,
        rowStart: 5,
        rowCount: 1,
        colStart: 0,
        colCount: 1,
      });
      if (g.cells?.[0]?.[0] !== 9.9) throw new Error("failed to set marker score");
      return { studentId };
    }, { classId, markSetId });

    await page.getByTestId("nav-students").click();
    await page.waitForSelector('[data-testid="students-screen"]');

    for (let i = 0; i < 5; i++) {
      await page.getByTestId(`student-move-up-${studentId}`).click();
    }

    // Remount Marks screen so it reloads the new sort_order mapping.
    await page.getByTestId("nav-marks").click();
    await page.waitForSelector('[data-testid="marks-screen"]');

    const v = await page.evaluate(async ({ workspacePath, classId, markSetId }) => {
      // Make sure we're still pointing at the same temp workspace.
      await window.markbook.request("workspace.select", { path: workspacePath });
      const g = await window.markbook.request("grid.get", {
        classId,
        markSetId,
        rowStart: 0,
        rowCount: 1,
        colStart: 0,
        colCount: 1,
      });
      return g.cells[0][0];
    }, { workspacePath, classId, markSetId });

    expect(v).toBeCloseTo(9.9, 5);
  } finally {
    await app.close();
  }
});
