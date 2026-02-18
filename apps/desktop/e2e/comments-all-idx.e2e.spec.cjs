const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("ALL! IDX combined comment sets are merged into mark sets", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const sets = await page.evaluate(async ({ classId, markSetId }) => {
      return await window.markbook.request("comments.sets.list", { classId, markSetId });
    }, { classId, markSetId });

    expect(Array.isArray(sets.sets)).toBeTruthy();
    expect(sets.sets.length).toBeGreaterThanOrEqual(2);
    const hasCombinedTitle = sets.sets.some((s) =>
      String(s.title || "").toLowerCase().includes("combined")
    );
    expect(hasCombinedTitle).toBeTruthy();
  } finally {
    await app.close();
  }
});
