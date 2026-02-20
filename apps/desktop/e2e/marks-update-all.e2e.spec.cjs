const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("update all applies a scored value to selected student across visible entries", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(
      page,
      repoRoot,
      "fixtures/legacy/Sample25/MB8D25"
    );

    await page.getByTestId("nav-marks").click();
    await expect(page.getByTestId("marks-screen")).toBeVisible();

    await expect
      .poll(async () => {
        const vis = await page.evaluate(() => {
          return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { sourceIdxs: [] };
        });
        return vis.sourceIdxs.length;
      })
      .toBeGreaterThan(2);

    await page.evaluate(() => {
      const ok = window.__markbookTest?.openMarksCellEditor?.(1, 0);
      if (!ok) throw new Error("failed to select row for update all");
    });
    await page.keyboard.press("Escape");

    const visible = await page.evaluate(() => {
      return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { sourceIdxs: [] };
    });
    expect(visible.sourceIdxs.length).toBeGreaterThan(2);

    await page.getByTestId("marks-action-update-all-btn").click();
    await expect(page.getByTestId("marks-update-all-modal")).toBeVisible();
    await page.locator('[data-testid="marks-update-all-apply-btn"]').click();
    await expect(page.getByTestId("marks-update-all-modal")).toBeHidden();

    const probeCols = visible.sourceIdxs.slice(0, 3);
    const probeValues = await page.evaluate(async ({ classId, markSetId, cols }) => {
      const out = [];
      for (const sourceCol of cols) {
        const grid = await window.markbook.request("grid.get", {
          classId,
          markSetId,
          rowStart: 0,
          rowCount: 1,
          colStart: sourceCol,
          colCount: 1
        });
        out.push(grid.cells?.[0]?.[0] ?? null);
      }
      return out;
    }, { classId: imported.classId, markSetId: imported.markSetId, cols: probeCols });

    for (const v of probeValues) {
      expect(v).toBe(1);
    }
  } finally {
    await app.close();
  }
});
