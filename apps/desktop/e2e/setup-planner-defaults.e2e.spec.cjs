const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("setup planner defaults persist and are readable", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId(`class-btn-${imported.classId}`).click();
    await page.getByTestId("nav-setup-admin").click();
    await page.waitForSelector('[data-testid="setup-admin-screen"]');
    await expect(page.getByTestId("setup-course-weeks")).toHaveValue("36");

    await page.getByTestId("setup-planner-duration").fill("95");
    await page.getByTestId("setup-planner-publish-status").selectOption("published");
    await page.getByTestId("setup-save-planner").click();

    await page.getByTestId("setup-course-weeks").fill("40");
    await page.getByTestId("setup-save-course-description").click();

    await page.getByTestId("setup-reports-planner-header-style").selectOption("compact");
    await page.getByTestId("setup-save-reports").click();

    const persisted = await page.evaluate(async () => window.markbook.request("setup.get", {}));
    expect(persisted.planner.defaultLessonDurationMinutes).toBe(95);
    expect(persisted.planner.defaultPublishStatus).toBe("published");
    expect(persisted.courseDescription.defaultTotalWeeks).toBe(40);
    expect(persisted.reports.plannerHeaderStyle).toBe("compact");
  } finally {
    await app.close();
  }
});
