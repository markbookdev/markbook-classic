const { test, expect } = require("@playwright/test");
const { launchElectronApp, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");

test("mark set transfer preview/apply imports assessments into target mark set", async () => {
  test.setTimeout(90_000);
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const fixture = path.join(repoRoot, "fixtures", "legacy", "Sample25", "MB8D25");
    const workspacePath = mkdtemp("markbook-transfer-workspace-");
    const setup = await page.evaluate(async ({ workspacePath, fixture }) => {
      await window.markbook.request("workspace.select", { path: workspacePath });
      const a = await window.markbook.request("class.importLegacy", {
        legacyClassFolderPath: fixture
      });
      const b = await window.markbook.request("class.importLegacy", {
        legacyClassFolderPath: fixture
      });
      const listA = await window.markbook.request("marksets.list", { classId: a.classId });
      const listB = await window.markbook.request("marksets.list", { classId: b.classId });
      const src = listA.markSets.find((m) => m.code === "MAT1") ?? listA.markSets[0];
      const tgt = listB.markSets.find((m) => m.code === "MAT1") ?? listB.markSets[0];
      const before = await window.markbook.request("assessments.list", {
        classId: b.classId,
        markSetId: tgt.id,
        hideDeleted: false
      });
      return {
        sourceClassId: a.classId,
        sourceMarkSetId: src.id,
        targetClassId: b.classId,
        targetMarkSetId: tgt.id,
        targetBeforeCount: before.assessments.length
      };
    }, { workspacePath, fixture });

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId(`class-btn-${setup.targetClassId}`).click();
    await page.getByTestId(`markset-btn-${setup.targetMarkSetId}`).click();
    await page.getByTestId("nav-markset-setup").click();
    await expect(page.getByTestId("markset-setup-screen")).toBeVisible();

    await page.getByTestId("markset-transfer-open-btn").click();
    await expect(page.getByTestId("markset-transfer-modal")).toBeVisible();

    await page
      .locator('[data-testid="markset-transfer-modal"] select')
      .nth(0)
      .selectOption(setup.sourceClassId);
    await page
      .locator('[data-testid="markset-transfer-modal"] select')
      .nth(1)
      .selectOption(setup.sourceMarkSetId);
    await page
      .locator('[data-testid="markset-transfer-modal"] select')
      .nth(2)
      .selectOption("append_new");

    await page.getByTestId("markset-transfer-preview-btn").click();
    await expect(page.getByTestId("markset-transfer-preview-summary")).toBeVisible();
    await page.getByTestId("markset-transfer-apply-btn").click();
    await expect(page.getByTestId("markset-transfer-modal")).toBeHidden();

    const afterCount = await page.evaluate(async ({ classId, markSetId }) => {
      const after = await window.markbook.request("assessments.list", {
        classId,
        markSetId,
        hideDeleted: false
      });
      return after.assessments.length;
    }, { classId: setup.targetClassId, markSetId: setup.targetMarkSetId });

    expect(afterCount).toBeGreaterThan(setup.targetBeforeCount);
  } finally {
    await app.close();
  }
});
