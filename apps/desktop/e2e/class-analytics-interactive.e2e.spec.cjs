const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("class analytics interactive controls drive rows and drilldown", async () => {
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

    await page.getByTestId("nav-class-analytics").click();
    await expect(page.getByTestId("class-analytics-screen")).toBeVisible();

    await page.getByTestId("analytics-filter-scope").selectOption("valid");
    await page.getByTestId("analytics-filter-term").selectOption("1");

    const rowsResp = await page.evaluate(async ({ classId, markSetId }) => {
      return await window.markbook.request("analytics.class.rows", {
        classId,
        markSetId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid",
        query: {
          search: null,
          sortBy: "sortOrder",
          sortDir: "asc",
          page: 1,
          pageSize: 25
        }
      });
    }, { classId, markSetId });
    expect(rowsResp.totalRows).toBeGreaterThanOrEqual(1);

    const cohortBins = page.locator('[data-testid^="analytics-class-bin-filter-"]');
    await expect(cohortBins.first()).toBeVisible({ timeout: 90000 });
    if ((await cohortBins.count()) > 0) {
      await cohortBins.first().click();
      await page.waitForTimeout(200);
    }

    const drilldownButtons = page.locator('[data-testid^="analytics-assessment-drilldown-open-"]');
    await expect(drilldownButtons.first()).toBeVisible();
    await drilldownButtons.first().click();
    await expect(page.getByTestId("analytics-assessment-drilldown-panel")).toBeVisible();
  } finally {
    await app.close();
  }
});
