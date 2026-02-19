const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("mark set manager create/clone/delete/undelete/default lifecycle", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(
      page,
      repoRoot,
      "fixtures/legacy/Sample25/MB8D25"
    );

    await page.getByTestId("nav-markset-setup").click();
    await expect(page.getByTestId("markset-setup-screen")).toBeVisible();

    await page.getByTestId("markset-manager-new-code").fill("ZZ1");
    await page.getByTestId("markset-manager-new-description").fill("ZZ Mark Set");
    await page.getByTestId("markset-manager-new-block-title").fill("Term");
    await page.getByTestId("markset-manager-create-btn").click();

    let created = null;
    let listAfterCreate = null;
    for (let i = 0; i < 25; i += 1) {
      listAfterCreate = await page.evaluate(async ({ classId }) => {
        return await window.markbook.request("marksets.list", {
          classId,
          includeDeleted: true
        });
      }, { classId: imported.classId });
      created = listAfterCreate.markSets.find((m) => m.code === "ZZ1") ?? null;
      if (created) break;
      await page.waitForTimeout(200);
    }
    expect(created).toBeTruthy();
    const countAfterCreate = listAfterCreate.markSets.length;

    await page.getByTestId(`markset-manager-clone-${created.id}`).click();

    let cloned = null;
    let listAfterClone = null;
    for (let i = 0; i < 25; i += 1) {
      listAfterClone = await page.evaluate(async ({ classId }) => {
        return await window.markbook.request("marksets.list", {
          classId,
          includeDeleted: true
        });
      }, { classId: imported.classId });
      cloned =
        listAfterClone.markSets.find(
          (m) => m.id !== created.id && String(m.description ?? "").includes("(Copy)")
        ) ?? null;
      if (cloned && listAfterClone.markSets.length >= countAfterCreate + 1) break;
      await page.waitForTimeout(200);
    }
    expect(listAfterClone.markSets.length).toBeGreaterThanOrEqual(countAfterCreate + 1);
    expect(cloned).toBeTruthy();

    page.once("dialog", (d) => d.accept());
    await page.getByTestId(`markset-manager-delete-${created.id}`).click();
    await page.getByTestId(`markset-manager-undelete-${created.id}`).click();
    await page.getByTestId(`markset-manager-default-${created.id}`).click();

    const listAfterDefault = await page.evaluate(async ({ classId, markSetId }) => {
      return await window.markbook.request("marksets.list", {
        classId,
        includeDeleted: true
      });
    }, { classId: imported.classId, markSetId: created.id });
    const defaultRow = listAfterDefault.markSets.find((m) => m.id === created.id);
    expect(defaultRow?.isDefault).toBeTruthy();
  } finally {
    await app.close();
  }
});
