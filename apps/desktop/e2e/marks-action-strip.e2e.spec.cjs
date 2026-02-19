const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("marks action strip supports legacy quick actions", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(
      page,
      repoRoot,
      "fixtures/legacy/Sample25/MB8D25"
    );

    await page.getByTestId("nav-marks").click();
    await expect(page.getByTestId("marks-screen")).toBeVisible();
    await expect(page.getByTestId("marks-bulk-toolbar")).toBeVisible();

    const before = await page.evaluate(async ({ classId, markSetId }) => {
      const list = await window.markbook.request("assessments.list", { classId, markSetId });
      return list.assessments.length;
    }, { classId: imported.classId, markSetId: imported.markSetId });

    await page.getByTestId("marks-action-new-entry-btn").click();
    let afterOne = before;
    for (let i = 0; i < 25; i += 1) {
      afterOne = await page.evaluate(async ({ classId, markSetId }) => {
        const list = await window.markbook.request("assessments.list", { classId, markSetId });
        return list.assessments.length;
      }, { classId: imported.classId, markSetId: imported.markSetId });
      if (afterOne >= before + 1) break;
      await page.waitForTimeout(200);
    }
    expect(afterOne).toBe(before + 1);

    await page.getByTestId("marks-action-multiple-new-btn").click();

    let afterBulk = afterOne;
    for (let i = 0; i < 25; i += 1) {
      afterBulk = await page.evaluate(async ({ classId, markSetId }) => {
        const list = await window.markbook.request("assessments.list", { classId, markSetId });
        return list.assessments.length;
      }, { classId: imported.classId, markSetId: imported.markSetId });
      if (afterBulk >= afterOne + 2) break;
      await page.waitForTimeout(200);
    }
    expect(afterBulk).toBe(afterOne + 2);

    // Ensure a mark entry column is selected for legacy quick actions.
    await page.evaluate(() => {
      const ok = window.__markbookTest?.openMarksCellEditor?.(1, 0);
      if (!ok) throw new Error("failed to select first mark entry cell");
    });
    await page.keyboard.press("Escape");

    await page.getByTestId("marks-action-entry-update-btn").click();
    await expect(page.getByTestId("marks-assessment-update-modal")).toBeVisible();
    await page.getByTestId("marks-assessment-update-cancel-btn").click();

    await page.getByTestId("marks-action-multiple-update-btn").click();
    await expect(page.getByTestId("marks-assessment-update-modal")).toBeVisible();
    await page.getByTestId("marks-assessment-update-apply-btn").click();

    let firstWeight = null;
    for (let i = 0; i < 25; i += 1) {
      firstWeight = await page.evaluate(async ({ classId, markSetId }) => {
        const list = await window.markbook.request("assessments.list", { classId, markSetId });
        return list.assessments[0]?.weight ?? null;
      }, { classId: imported.classId, markSetId: imported.markSetId });
      if (firstWeight === 2) break;
      await page.waitForTimeout(200);
    }
    expect(firstWeight).toBe(2);
  } finally {
    await app.close();
  }
});
