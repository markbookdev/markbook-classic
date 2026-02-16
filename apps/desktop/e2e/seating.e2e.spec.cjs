const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("seating assignments and blocked seats persist", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );

    const student = await page.evaluate(async ({ classId }) => {
      const res = await window.markbook.request("students.list", { classId });
      return res.students[0];
    }, { classId });
    expect(student?.id).toBeTruthy();

    await page.getByTestId("nav-seating").click();
    await page.waitForSelector('[data-testid="seating-screen"]');

    await page.getByTestId(`seating-student-${student.id}`).click();
    await page.getByTestId("seating-seat-assign-1").click();
    await page.getByTestId("seating-block-toggle-2").click();
    await page.getByTestId("seating-save-btn").click();
    await page.getByTestId("seating-reload-btn").click();

    const persisted = await page.evaluate(async ({ classId }) => {
      return await window.markbook.request("seating.get", { classId });
    }, { classId });

    expect(persisted.assignments[0]).toBe(student.sortOrder);
    expect(persisted.blockedSeatCodes.includes(2)).toBeTruthy();
  } finally {
    await app.close();
  }
});

