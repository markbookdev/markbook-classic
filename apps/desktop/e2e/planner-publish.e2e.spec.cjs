const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("planner publish preview and commit", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    // Seed at least one unit for publish source.
    await page.evaluate(async ({ classId }) => {
      await window.markbook.request("planner.units.create", {
        classId,
        input: { title: "Publish Unit" }
      });
    }, { classId: imported.classId });

    await page.getByTestId("nav-planner").click();
    await page.waitForSelector('[data-testid="planner-screen"]');
    await page.getByTestId("planner-publish-tab").click();

    await page.getByTestId("planner-publish-preview-btn").click();
    const titleInput = page.getByPlaceholder("Publish title");
    await expect(titleInput).toHaveValue(/.+/);
    await page.getByTestId("planner-publish-commit-btn").click();

    await expect(page.getByText("Published artifact saved.")).toBeVisible();
    await expect(page.locator("tr", { hasText: "Publish Unit" })).toBeVisible();
  } finally {
    await app.close();
  }
});
