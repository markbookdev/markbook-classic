const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("menu action labels stay consistent with target screen header labels", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");
    await page.getByTestId(`class-btn-${imported.classId}`).click();

    const groups = page.getByTestId("legacy-menu-groups");
    await expect(groups).toBeVisible();

    await groups.locator('summary:has-text("Working On")').click();
    await groups.getByRole("button", { name: "Edit Marks" }).click();
    await expect(page.getByTestId("marks-screen")).toBeVisible();
    await expect(page.getByTestId("marks-screen-header-label")).toHaveText("Edit Marks");

    await groups.locator('summary:has-text("Reports")').click();
    await groups.getByRole("button", { name: "Mark Set Reports" }).click();
    await expect(page.getByTestId("reports-screen")).toBeVisible();
    await expect(page.getByTestId("reports-screen-header-label")).toHaveText("Mark Set Reports");

    await groups.locator('summary:has-text("Integrations")').click();
    await groups.getByRole("button", { name: "Class Exchange" }).click();
    await expect(page.getByTestId("exchange-screen")).toBeVisible();
    await expect(page.getByTestId("exchange-screen-header-label")).toHaveText("Class Exchange");

    await groups.locator('summary:has-text("Planner")').click();
    await groups.getByRole("button", { name: "Units + Lessons" }).click();
    await expect(page.getByTestId("planner-screen")).toBeVisible();
    await expect(page.getByTestId("planner-screen-header-label")).toHaveText("Units + Lessons");
  } finally {
    await app.close();
  }
});
