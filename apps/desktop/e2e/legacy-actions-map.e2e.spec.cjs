const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("legacy actions map screen lists route/status for menu actions", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");
    await page.getByTestId(`class-btn-${imported.classId}`).click();

    const groups = page.getByTestId("legacy-menu-groups");
    await expect(groups).toBeVisible();
    await groups.locator('summary:has-text("Help")').click();
    await groups.getByRole("button", { name: "Legacy Actions Map" }).click();

    const screen = page.getByTestId("legacy-actions-map-screen");
    await expect(screen).toBeVisible();
    await expect(page.getByTestId("legacy-actions-map-title")).toBeVisible();

    await expect(page.getByTestId("legacy-action-map-row-file.select_printer")).toBeVisible();
    await expect(page.getByTestId("legacy-action-map-row-class.email_class_list")).toBeVisible();
    await expect(page.getByTestId("legacy-action-map-row-workingon.edit_marks")).toBeVisible();
  } finally {
    await app.close();
  }
});
