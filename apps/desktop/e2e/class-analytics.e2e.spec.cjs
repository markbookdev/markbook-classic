const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

function parseMark(text) {
  const t = String(text ?? "").trim();
  if (t === "—" || t.length === 0) return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

test("class analytics screen loads and aligns KPI with sidecar for selected filters", async () => {
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

    const expected = await page.evaluate(async ({ classId, markSetId }) => {
      const model = await window.markbook.request("analytics.class.open", {
        classId,
        markSetId,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
      return model?.kpis?.classAverage ?? null;
    }, { classId, markSetId });

    const displayed = parseMark(
      await page.getByTestId("class-analytics-kpi-average-value").innerText()
    );
    if (expected == null) {
      expect(displayed).toBeNull();
    } else {
      expect(displayed).toBeCloseTo(expected, 1);
    }
  } finally {
    await app.close();
  }
});
