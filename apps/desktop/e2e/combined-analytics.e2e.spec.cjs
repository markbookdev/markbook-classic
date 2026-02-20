const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

function parseMark(text) {
  const t = String(text ?? "").trim();
  if (t === "—" || t.length === 0) return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

test("combined analytics screen loads and aligns KPI average with sidecar model", async () => {
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
    await page.getByTestId("combined-analytics-filter-scope").selectOption("valid");
    await page.getByTestId("combined-analytics-filter-term").selectOption("1");
    await page.waitForTimeout(200);

    const selectedMarkSetIds = await page.evaluate(() => {
      const sel = document.querySelector('[data-testid="combined-analytics-markset-multiselect"]');
      if (!sel) return [];
      return Array.from(sel.selectedOptions).map((o) => o.value);
    });
    expect(Array.isArray(selectedMarkSetIds)).toBeTruthy();
    expect(selectedMarkSetIds.length).toBeGreaterThan(0);

    const expected = await page.evaluate(async ({ classId, markSetIds }) => {
      const model = await window.markbook.request("analytics.combined.open", {
        classId,
        markSetIds,
        filters: { term: 1, categoryName: null, typesMask: null },
        studentScope: "valid"
      });
      return model?.kpis?.classAverage ?? null;
    }, { classId, markSetIds: selectedMarkSetIds });

    const displayed = parseMark(
      await page.getByTestId("combined-analytics-kpi-average-value").innerText()
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

