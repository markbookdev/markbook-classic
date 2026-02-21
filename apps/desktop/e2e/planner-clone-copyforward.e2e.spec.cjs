const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("planner clone unit and copy-forward lesson workflows", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId("nav-planner").click();
    await page.waitForSelector('[data-testid="planner-screen"]');
    await page.getByTestId("planner-units-tab").click();

    await page.getByPlaceholder("New unit title").fill("Wave9 Unit");
    await page.getByTestId("planner-unit-create-btn").click();
    const unitRow = page.locator("tr", { hasText: "Wave9 Unit" }).first();
    await expect(unitRow).toBeVisible();
    await unitRow.getByRole("button", { name: "Clone" }).click();
    await expect(page.locator("tr", { hasText: "Wave9 Unit (Copy)" }).first()).toBeVisible();

    await page.getByTestId("planner-lessons-tab").click();
    await page.getByPlaceholder("New lesson title").fill("Wave9 Lesson");
    await page.locator('input[type="date"]').first().fill("2026-02-21");
    await page.getByPlaceholder("Minutes").fill("65");
    await page.getByTestId("planner-lesson-create-btn").click();

    const lessonRow = page.locator("tr", { hasText: "Wave9 Lesson" }).first();
    await expect(lessonRow).toBeVisible();
    await lessonRow.locator('input[type="checkbox"]').check();
    await page.getByTestId("planner-lessons-copyforward-day-offset").fill("2");
    await page.getByTestId("planner-lessons-copyforward-btn").click();
    await expect(page.getByText("Copied 1 lesson(s) forward.")).toBeVisible();
    await expect(page.locator("tr", { hasText: "Wave9 Lesson" })).toHaveCount(2);

    await page.getByTestId("planner-lessons-bulk-assign-unit-select").selectOption({
      label: "Wave9 Unit (Copy)"
    });
    await page.getByTestId("planner-lessons-bulk-assign-btn").click();
    await expect(page.getByText("Updated 1 lesson assignment(s).")).toBeVisible();
  } finally {
    await app.close();
  }
});
