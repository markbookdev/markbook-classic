const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("hide deleted entries toggle affects visible marks columns", async () => {
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
          return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { ids: [] };
        });
        return vis.ids.length;
      })
      .toBeGreaterThan(0);

    const before = await page.evaluate(() => {
      return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { ids: [] };
    });
    expect(before.ids.length).toBeGreaterThan(0);
    const deletedCandidateId = before.ids[0];

    await page.evaluate(() => {
      const ok = window.__markbookTest?.openMarksCellEditor?.(1, 0);
      if (!ok) throw new Error("failed to select first entry");
    });
    await page.keyboard.press("Escape");

    page.once("dialog", async (d) => d.accept());
    await page.getByTestId("marks-action-delete-entry-btn").click();

    let afterDelete = before;
    for (let i = 0; i < 25; i += 1) {
      afterDelete = await page.evaluate(() => {
        return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { ids: [] };
      });
      if (!afterDelete.ids.includes(deletedCandidateId)) break;
      await page.waitForTimeout(200);
    }
    expect(afterDelete.ids).not.toContain(deletedCandidateId);
    expect(afterDelete.hideDeletedEntries).toBe(true);

    await page.getByTestId("marks-action-hide-deleted-btn").click();
    let afterShow = afterDelete;
    for (let i = 0; i < 25; i += 1) {
      afterShow = await page.evaluate(() => {
        return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { ids: [] };
      });
      if (afterShow.ids.includes(deletedCandidateId)) break;
      await page.waitForTimeout(200);
    }
    expect(afterShow.hideDeletedEntries).toBe(false);
    expect(afterShow.ids).toContain(deletedCandidateId);

    await page.getByTestId("marks-action-hide-deleted-btn").click();
    let afterHideAgain = afterShow;
    for (let i = 0; i < 25; i += 1) {
      afterHideAgain = await page.evaluate(() => {
        return window.__markbookTest?.getMarksVisibleAssessments?.() ?? { ids: [] };
      });
      if (!afterHideAgain.ids.includes(deletedCandidateId)) break;
      await page.waitForTimeout(200);
    }
    expect(afterHideAgain.hideDeletedEntries).toBe(true);
    expect(afterHideAgain.ids).not.toContain(deletedCandidateId);

    const pref = await page.evaluate(async ({ classId, markSetId }) => {
      return await window.markbook.request("marks.pref.hideDeleted.get", { classId, markSetId });
    }, { classId: imported.classId, markSetId: imported.markSetId });
    expect(pref.hideDeleted).toBe(true);
  } finally {
    await app.close();
  }
});
