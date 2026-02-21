const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("legacy menu groups are discoverable and route to implemented screens", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");
    await page.getByTestId(`class-btn-${imported.classId}`).click();

    const groups = page.getByTestId("legacy-menu-groups");
    await expect(groups).toBeVisible();
    for (const title of ["File", "Class", "Mark Sets", "Working On", "Reports", "Comments", "Setup"]) {
      await expect(groups.locator(`summary:has-text("${title}")`)).toBeVisible();
    }

    await groups.locator('summary:has-text("Reports")').click();
    await groups.getByRole("button", { name: "Combined Analytics" }).click();
    await expect(page.getByTestId("combined-analytics-screen")).toBeVisible();

    await groups.locator('summary:has-text("Comments")').click();
    await groups.getByRole("button", { name: "Remarks in Marks" }).click();
    await expect(page.getByTestId("marks-screen")).toBeVisible();
  } finally {
    await app.close();
  }
});
