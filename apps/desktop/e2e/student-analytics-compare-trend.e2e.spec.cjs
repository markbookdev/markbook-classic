const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("student analytics compare/trend panels load and honor trend selection", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();
    await page.getByTestId(`markset-btn-${markSetId}`).click();

    await page.getByTestId("nav-student-analytics").click();
    await expect(page.getByTestId("student-analytics-screen")).toBeVisible();
    await expect(page.getByTestId("analytics-student-compare-panel")).toBeVisible();
    await expect(page.getByTestId("analytics-student-trend-panel")).toBeVisible();

    await page.getByTestId("analytics-filter-scope").selectOption("valid");
    await page.getByTestId("analytics-filter-term").selectOption("1");
    await page.waitForTimeout(200);

    const selectedStudentId = await page.getByTestId("analytics-student-select").inputValue();
    expect(selectedStudentId).toBeTruthy();

    const chosenMarkSetIds = await page.evaluate(() => {
      const select = document.querySelector('[data-testid="analytics-student-trend-marksets"]');
      if (!select) return [];
      const options = Array.from(select.options);
      for (const option of options) option.selected = false;
      for (const option of options.slice(0, 2)) option.selected = true;
      select.dispatchEvent(new Event("change", { bubbles: true }));
      return options.slice(0, 2).map((o) => o.value);
    });
    expect(chosenMarkSetIds.length).toBeGreaterThanOrEqual(1);
    await page.waitForTimeout(250);

    const expectedTrend = await page.evaluate(async ({ classId, studentId, markSetIds }) => {
      return await window.markbook.request("analytics.student.trend", {
        classId,
        studentId,
        markSetIds,
        filters: { term: 1, categoryName: null, typesMask: null }
      });
    }, {
      classId,
      studentId: selectedStudentId,
      markSetIds: chosenMarkSetIds
    });

    const renderedRows = await page
      .getByTestId("analytics-student-trend-panel")
      .locator("tbody tr")
      .count();
    expect(renderedRows).toBe(expectedTrend.points.length);

    const expectedCompare = await page.evaluate(async ({ classId, markSetId, studentId }) => {
      return await window.markbook.request("analytics.student.compare", {
        classId,
        markSetId,
        studentId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
    }, { classId, markSetId, studentId: selectedStudentId });
    expect(typeof expectedCompare.cohort.studentCount).toBe("number");
  } finally {
    await app.close();
  }
});
