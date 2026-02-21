const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("course description profile and model generation", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId("nav-course-description").click();
    await page.waitForSelector('[data-testid="course-description-screen"]');

    const models = await page.evaluate(async ({ classId }) => {
      await window.markbook.request("courseDescription.updateProfile", {
        classId,
        patch: { courseTitle: "E2E Course Title", gradeLabel: "Grade 10" }
      });
      const generated = await window.markbook.request("courseDescription.generateModel", {
        classId,
        options: { includePolicy: true }
      });
      const time = await window.markbook.request("courseDescription.timeManagementModel", {
        classId
      });
      return { generated, time };
    }, { classId: imported.classId });

    expect(models.generated.profile.courseTitle).toBe("E2E Course Title");
    expect(models.time.totals.availableMinutes).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
