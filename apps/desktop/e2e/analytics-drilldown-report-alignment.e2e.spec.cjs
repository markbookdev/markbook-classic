const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("class drilldown handoff to reports stays aligned", async () => {
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
    await page.waitForTimeout(150);

    const drilldownBtn = page.locator('[data-testid^="analytics-assessment-drilldown-open-"]').first();
    await expect(drilldownBtn).toBeVisible();
    const drilldownTestId = await drilldownBtn.getAttribute("data-testid");
    const assessmentId = String(drilldownTestId || "").replace(
      "analytics-assessment-drilldown-open-",
      ""
    );
    expect(assessmentId.length).toBeGreaterThan(0);
    await drilldownBtn.click();
    await expect(page.getByTestId("analytics-assessment-drilldown-panel")).toBeVisible();

    await page.getByRole("button", { name: "Open Drilldown in Reports" }).click();
    await expect(page.getByTestId("reports-screen")).toBeVisible();
    await expect(page.getByTestId("reports-filter-student-scope")).toHaveValue("valid");
    await expect(page.getByTestId("reports-filter-term")).toHaveValue("1");
    await expect(page.getByTestId("export-class-assessment-drilldown-pdf-btn")).toBeEnabled();

    const compare = await page.evaluate(async ({ classId, markSetId, assessmentId }) => {
      const params = {
        classId,
        markSetId,
        assessmentId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid",
        query: {
          search: null,
          sortBy: "sortOrder",
          sortDir: "asc",
          page: 1,
          pageSize: 25
        }
      };
      const analytics = await window.markbook.request("analytics.class.assessmentDrilldown", params);
      const report = await window.markbook.request("reports.classAssessmentDrilldownModel", params);
      return {
        analyticsAvgPercent: analytics?.classStats?.avgPercent ?? null,
        reportAvgPercent: report?.classStats?.avgPercent ?? null,
        analyticsTotalRows: analytics?.totalRows ?? null,
        reportTotalRows: report?.totalRows ?? null
      };
    }, { classId, markSetId, assessmentId });

    expect(compare.analyticsTotalRows).toBe(compare.reportTotalRows);
    if (compare.analyticsAvgPercent == null || compare.reportAvgPercent == null) {
      expect(compare.analyticsAvgPercent).toBe(compare.reportAvgPercent);
    } else {
      expect(compare.analyticsAvgPercent).toBeCloseTo(compare.reportAvgPercent, 4);
    }
  } finally {
    await app.close();
  }
});
