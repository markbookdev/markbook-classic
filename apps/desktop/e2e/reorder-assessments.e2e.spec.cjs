const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("assessment reorder updates marks column mapping (UI -> persisted)", async () => {
  const { app, page, repoRoot } = await launchElectronApp();

  try {
    const { workspacePath, classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const { assessmentId } = await page.evaluate(async ({ classId, markSetId }) => {
      const open = await window.markbook.request("markset.open", { classId, markSetId });
      const assessmentId = open.assessments[0]?.id;
      if (!assessmentId) throw new Error("no assessments in fixture");

      // Set a unique value at row 0 / col 0 (assessment idx 0).
      await window.markbook.request("grid.updateCell", {
        classId,
        markSetId,
        row: 0,
        col: 0,
        value: 9.8,
        editKind: "set",
      });
      return { assessmentId };
    }, { classId, markSetId });

    await page.getByTestId("nav-markset-setup").click();
    await page.waitForSelector('[data-testid="markset-setup-screen"]');

    // Move the first assessment down one position.
    await page.getByTestId(`assessment-move-down-${assessmentId}`).click();

    // Verify the score moved from col 0 to col 1 after reorder.
    const cells = await page.evaluate(async ({ workspacePath, classId, markSetId }) => {
      await window.markbook.request("workspace.select", { path: workspacePath });
      const g = await window.markbook.request("grid.get", {
        classId,
        markSetId,
        rowStart: 0,
        rowCount: 1,
        colStart: 0,
        colCount: 2,
      });
      return g.cells[0];
    }, { workspacePath, classId, markSetId });

    expect(cells[1]).toBeCloseTo(9.8, 5);
  } finally {
    await app.close();
  }
});

