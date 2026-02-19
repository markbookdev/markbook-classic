const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("grid.bulkUpdate persists multi-cell edits", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const payload = {
      classId,
      markSetId,
      edits: [
        { row: 0, col: 0, state: "scored", value: 9.5 },
        { row: 0, col: 1, state: "zero", value: null },
        { row: 1, col: 0, state: "no_mark", value: null }
      ]
    };

    const updated = await page.evaluate(async (p) => {
      const res = await window.markbook.request("grid.bulkUpdate", p);
      return res.updated;
    }, payload);
    expect(updated).toBe(3);

    const grid = await page.evaluate(async ({ classId, markSetId }) => {
      return await window.markbook.request("grid.get", {
        classId,
        markSetId,
        rowStart: 0,
        rowCount: 2,
        colStart: 0,
        colCount: 2
      });
    }, { classId, markSetId });

    expect(grid.cells[0][0]).toBeCloseTo(9.5, 5);
    expect(grid.cells[0][1]).toBe(0);
    expect(grid.cells[1][0]).toBeNull();
  } finally {
    await app.close();
  }
});

test("grid.bulkUpdate returns deterministic diagnostics when payload exceeds limit", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const res = await page.evaluate(async ({ classId, markSetId }) => {
      const edits = Array.from({ length: 5001 }, () => ({
        row: 0,
        col: 0,
        state: "scored",
        value: 1
      }));
      return await window.markbook.request("grid.bulkUpdate", {
        classId,
        markSetId,
        edits
      });
    }, { classId, markSetId });

    expect(res.ok).toBe(true);
    expect(res.updated).toBe(0);
    expect(res.rejected).toBe(5001);
    expect(res.limitExceeded).toBe(true);
    expect(Array.isArray(res.errors)).toBeTruthy();
    expect(res.errors.length).toBe(1);
    expect(res.errors[0].code).toBe("too_many_edits");
  } finally {
    await app.close();
  }
});
