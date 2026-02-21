const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("planner units create and archive persists", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId("nav-planner").click();
    await page.waitForSelector('[data-testid="planner-screen"]');
    await page.getByTestId("planner-units-tab").click();

    await page.getByPlaceholder("New unit title").fill("E2E Unit A");
    await page.getByTestId("planner-unit-create-btn").click();

    const row = page.locator("tr", { hasText: "E2E Unit A" }).first();
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "Archive" }).click();

    await expect(page.locator("tr", { hasText: "E2E Unit A" })).toHaveCount(0);

    await page.getByTestId("planner-show-archived-toggle").check();
    await expect(page.locator("tr", { hasText: "E2E Unit A" }).first()).toBeVisible();
  } finally {
    await app.close();
  }
});
