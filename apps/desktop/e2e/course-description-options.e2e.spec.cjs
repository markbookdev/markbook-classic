const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("course description generation options toggle model sections", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId("nav-course-description").click();
    await page.waitForSelector('[data-testid="course-description-screen"]');

    const firstModel = await page.evaluate(async ({ classId }) => {
      return await window.markbook.request("courseDescription.generateModel", {
        classId,
        options: {
          includePolicy: false,
          includeStrands: false,
          includeAssessmentPlan: false,
          includeResources: false
        }
      });
    }, { classId: imported.classId });
    expect(firstModel.profile.policyText).toBe("");
    expect(Array.isArray(firstModel.profile.strands) ? firstModel.profile.strands.length : -1).toBe(0);
    expect(Object.prototype.hasOwnProperty.call(firstModel, "assessmentPlan")).toBe(false);
    expect(Array.isArray(firstModel.resources) ? firstModel.resources.length : -1).toBe(0);

    const secondModel = await page.evaluate(async ({ classId }) => {
      return await window.markbook.request("courseDescription.generateModel", {
        classId,
        options: {
          includePolicy: true,
          includeStrands: true,
          includeAssessmentPlan: true,
          includeResources: true
        }
      });
    }, { classId: imported.classId });
    expect(Object.prototype.hasOwnProperty.call(secondModel, "assessmentPlan")).toBe(true);
    expect(Array.isArray(secondModel.resources)).toBe(true);
  } finally {
    await app.close();
  }
});
