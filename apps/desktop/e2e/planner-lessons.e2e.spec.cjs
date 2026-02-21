const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("planner lessons create and archive", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId("nav-planner").click();
    await page.waitForSelector('[data-testid="planner-screen"]');
    await page.getByTestId("planner-lessons-tab").click();

    await page.getByPlaceholder("New lesson title").fill("E2E Lesson A");
    const dateInput = page.locator('input[type="date"]').first();
    await dateInput.fill("2026-02-21");
    await page.getByPlaceholder("Minutes").fill("65");
    await page.getByTestId("planner-lesson-create-btn").click();

    const row = page.locator("tr", { hasText: "E2E Lesson A" }).first();
    await expect(row).toBeVisible();
    await row.getByRole("button", { name: "Archive" }).click();
    await expect(page.locator("tr", { hasText: "E2E Lesson A" })).toHaveCount(0);

    await page.getByTestId("planner-show-archived-toggle").check();
    await expect(page.locator("tr", { hasText: "E2E Lesson A" }).first()).toBeVisible();
  } finally {
    await app.close();
  }
});
