const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("combined analytics open-in-reports keeps filters and aligns model output", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    await page.getByTestId("nav-combined-analytics").click();
    await page.waitForSelector('[data-testid="combined-analytics-screen"]');
    await page.waitForFunction(() => {
      const sel = document.querySelector('[data-testid="combined-analytics-markset-multiselect"]');
      return !!sel && sel.options && sel.options.length > 1;
    });

    const allMarkSetIds = await page.evaluate(() => {
      const sel = document.querySelector('[data-testid="combined-analytics-markset-multiselect"]');
      if (!sel) return [];
      return Array.from(sel.options).map((o) => o.value);
    });
    expect(allMarkSetIds.length).toBeGreaterThan(1);
    const chosen = allMarkSetIds.slice(0, 2);
    await page.getByTestId("combined-analytics-markset-multiselect").selectOption(chosen);
    await page.getByTestId("combined-analytics-filter-scope").selectOption("valid");
    await page.getByTestId("combined-analytics-filter-term").selectOption("1");
    await page.waitForTimeout(200);

    await page.getByTestId("combined-analytics-open-reports").click();
    await page.waitForSelector('[data-testid="reports-screen"]');
    await expect(page.getByTestId("reports-filter-student-scope")).toHaveValue("valid");
    await expect(page.getByTestId("reports-filter-term")).toHaveValue("1");

    const compare = await page.evaluate(async ({ classId, markSetIds }) => {
      const params = {
        classId,
        markSetIds,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      };
      const analytics = await window.markbook.request("analytics.combined.open", params);
      const report = await window.markbook.request("reports.combinedAnalysisModel", params);
      return {
        analyticsAvg: analytics?.kpis?.classAverage ?? null,
        reportAvg: report?.kpis?.classAverage ?? null,
        analyticsCount: analytics?.kpis?.finalMarkCount ?? null,
        reportCount: report?.kpis?.finalMarkCount ?? null
      };
    }, { classId, markSetIds: chosen });

    expect(compare.analyticsCount).toBe(compare.reportCount);
    if (compare.analyticsAvg == null || compare.reportAvg == null) {
      expect(compare.analyticsAvg).toBe(compare.reportAvg);
    } else {
      expect(compare.analyticsAvg).toBeCloseTo(compare.reportAvg, 1);
    }
  } finally {
    await app.close();
  }
});
