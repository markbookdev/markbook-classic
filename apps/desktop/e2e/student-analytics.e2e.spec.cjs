const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

function parseMark(text) {
  const t = String(text ?? "").trim();
  if (t === "—" || t.length === 0) return null;
  const n = Number(t);
  return Number.isFinite(n) ? n : null;
}

test("student analytics screen reflects selected student/filter final mark", async () => {
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
    await page.waitForSelector('[data-testid="student-analytics-screen"]');

    const selectedStudentId = await page.getByTestId("analytics-student-select").inputValue();
    expect(selectedStudentId).toBeTruthy();

    await page.getByTestId("analytics-filter-term").selectOption("1");
    await page.waitForTimeout(150);

    const expected = await page.evaluate(async ({ classId, markSetId, studentId }) => {
      const model = await window.markbook.request("analytics.student.open", {
        classId,
        markSetId,
        studentId,
        filters: { term: 1, categoryName: null, typesMask: null }
      });
      return model?.finalMark ?? null;
    }, { classId, markSetId, studentId: selectedStudentId });

    const displayed = parseMark(
      await page.getByTestId("student-analytics-final-mark-value").innerText()
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
