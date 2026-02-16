const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("loaned items screen can create and persist an item", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );

    await page.getByTestId("nav-loaned-items").click();
    await page.waitForSelector('[data-testid="loaned-screen"]');

    await page.getByTestId("loaned-new-btn").click();
    await page.getByTestId("loaned-student-select").selectOption({ index: 1 });
    await page.getByTestId("loaned-item-name-input").fill("Workbook A");
    await page.getByTestId("loaned-quantity-input").fill("1");
    await page.getByTestId("loaned-notes-input").fill("E2E test item");
    await page.getByTestId("loaned-save-btn").click();
    await expect
      .poll(async () => {
        return await page.evaluate(async ({ classId }) => {
          const res = await window.markbook.request("loaned.list", { classId });
          return res.items.some((item) => item.itemName === "Workbook A");
        }, { classId });
      })
      .toBeTruthy();
  } finally {
    await app.close();
  }
});
