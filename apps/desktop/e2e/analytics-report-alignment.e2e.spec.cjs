const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("open-in-reports preserves analytics filters and matches summary average", async () => {
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
    await page.waitForSelector('[data-testid="class-analytics-screen"]');
    await page.getByTestId("analytics-filter-scope").selectOption("valid");
    await page.getByTestId("analytics-filter-term").selectOption("1");
    await page.waitForTimeout(150);
    await page.getByTestId("class-analytics-open-reports").click();

    await page.waitForSelector('[data-testid="reports-screen"]');
    await expect(page.getByTestId("reports-filter-student-scope")).toHaveValue("valid");
    await expect(page.getByTestId("reports-filter-term")).toHaveValue("1");

    const compare = await page.evaluate(async ({ classId, markSetId }) => {
      const analytics = await window.markbook.request("analytics.class.open", {
        classId,
        markSetId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
      const summary = await window.markbook.request("reports.markSetSummaryModel", {
        classId,
        markSetId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
      const marks = Array.isArray(summary?.perStudent)
        ? summary.perStudent
            .map((s) => s.finalMark)
            .filter((v) => typeof v === "number")
        : [];
      const reportAvg =
        marks.length > 0
          ? marks.reduce((a, b) => a + b, 0) / marks.length
          : null;
      return {
        analyticsAvg: analytics?.kpis?.classAverage ?? null,
        reportAvg
      };
    }, { classId, markSetId });

    if (compare.analyticsAvg == null || compare.reportAvg == null) {
      expect(compare.analyticsAvg).toBe(compare.reportAvg);
    } else {
      expect(compare.analyticsAvg).toBeCloseTo(compare.reportAvg, 1);
    }
  } finally {
    await app.close();
  }
});
