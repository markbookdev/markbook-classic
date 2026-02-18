const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const fs = require("node:fs");
const path = require("node:path");

test("reports models and export honor marks-style filters and student scope", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const models = await page.evaluate(async ({ classId, markSetId }) => {
      const allSummary = await window.markbook.request("reports.markSetSummaryModel", {
        classId,
        markSetId,
        filters: { term: null, categoryName: null, typesMask: null },
        studentScope: "all"
      });
      const filteredSummary = await window.markbook.request("reports.markSetSummaryModel", {
        classId,
        markSetId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
      return {
        allCount: Array.isArray(allSummary.perStudent) ? allSummary.perStudent.length : 0,
        filteredCount: Array.isArray(filteredSummary.perStudent) ? filteredSummary.perStudent.length : 0,
        allAssessCount: Array.isArray(allSummary.perAssessment) ? allSummary.perAssessment.length : 0,
        filteredAssessCount: Array.isArray(filteredSummary.perAssessment)
          ? filteredSummary.perAssessment.length
          : 0,
        filteredTerm: filteredSummary?.filters?.term ?? null
      };
    }, { classId, markSetId });

    expect(models.filteredTerm).toBe(1);
    // Scope=valid should reduce or keep the student list; Sample25 has excluded students so this
    // should be strictly smaller and acts as a deterministic alignment check.
    expect(models.filteredCount).toBeLessThan(models.allCount);
    expect(models.filteredAssessCount).toBeLessThanOrEqual(models.allAssessCount);

    await page.getByTestId("nav-reports").click();
    await page.waitForSelector('[data-testid="reports-screen"]');
    await page.getByTestId("reports-filter-student-scope").selectOption("valid");
    await page.getByTestId("reports-filter-term").selectOption("1");

    const outDir = mkdtemp("markbook-pdf-report-filters-");
    const outPath = path.join(outDir, "category-analysis-filters.pdf");

    await page.evaluate(async ({ classId, markSetId, outPath }) => {
      await window.__markbookTest.exportCategoryAnalysisPdfToPath(classId, markSetId, outPath, {
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
    }, { classId, markSetId, outPath });

    for (let i = 0; i < 50; i += 1) {
      if (fs.existsSync(outPath) && fs.statSync(outPath).size > 0) break;
      // eslint-disable-next-line no-await-in-loop
      await new Promise((r) => setTimeout(r, 100));
    }
    expect(fs.existsSync(outPath)).toBeTruthy();
    expect(fs.statSync(outPath).size).toBeGreaterThan(1000);
  } finally {
    await app.close();
  }
});
