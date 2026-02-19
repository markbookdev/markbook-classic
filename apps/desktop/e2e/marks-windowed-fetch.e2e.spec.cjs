const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("marks grid fetches windows on demand and persists edits in fetched tiles", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const bootstrap = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(bootstrap.classId).toBeTruthy();
    expect(bootstrap.markSetId).toBeTruthy();

    const prep = await page.evaluate(async ({ classId, markSetId }) => {
      const open = await window.markbook.request("markset.open", { classId, markSetId });
      const existing = open.assessments.length;
      const target = 46;
      for (let i = existing; i < target; i += 1) {
        // eslint-disable-next-line no-await-in-loop
        await window.markbook.request("assessments.create", {
          classId,
          markSetId,
          title: `Perf Window ${i + 1}`,
          term: 1,
          weight: 1,
          outOf: 10
        });
      }
      const after = await window.markbook.request("markset.open", { classId, markSetId });
      return {
        assessmentCount: after.assessments.length,
        targetCol: Math.min(35, after.assessments.length)
      };
    }, {
      classId: bootstrap.classId,
      markSetId: bootstrap.markSetId
    });

    expect(prep.assessmentCount).toBeGreaterThanOrEqual(35);
    expect(prep.targetCol).toBeGreaterThan(0);

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId(`class-btn-${bootstrap.classId}`).click();
    await page.getByTestId(`markset-btn-${bootstrap.markSetId}`).click();
    await page.getByTestId("nav-marks").click();
    await page.waitForSelector('[data-testid="marks-screen"]');

    await page.waitForFunction(() => {
      const d = window.__markbookTest?.getMarksGridDebug?.();
      return !!d && d.gridGetRequests >= 1;
    });
    const before = await page.evaluate(() => window.__markbookTest.getMarksGridDebug());
    expect(before.gridGetRequests).toBeGreaterThanOrEqual(1);
    expect(before.tileRequests).toBeGreaterThanOrEqual(1);

    const opened = await page.evaluate(({ col }) => {
      return window.__markbookTest?.openMarksCellEditor?.(col, 0) ?? false;
    }, { col: prep.targetCol });
    expect(opened).toBeTruthy();
    await page.waitForSelector('[data-testid="mark-grid-editor-input"]');

    await page.waitForFunction((beforeCount) => {
      const d = window.__markbookTest?.getMarksGridDebug?.();
      return !!d && d.gridGetRequests > beforeCount;
    }, before.gridGetRequests);

    const afterFarCol = await page.evaluate(() => window.__markbookTest.getMarksGridDebug());
    expect(afterFarCol.tileCacheMisses).toBeGreaterThan(before.tileCacheMisses);
    expect(afterFarCol.inflightMax).toBeGreaterThanOrEqual(1);

    // Opening the same far cell again should hit cache (no extra network request).
    await page.evaluate(({ col }) => {
      return window.__markbookTest?.openMarksCellEditor?.(col, 0) ?? false;
    }, { col: prep.targetCol });
    await page.waitForFunction((beforeHits) => {
      const d = window.__markbookTest?.getMarksGridDebug?.();
      return !!d && d.tileCacheHits > beforeHits;
    }, afterFarCol.tileCacheHits);
    const afterSameCell = await page.evaluate(() => window.__markbookTest.getMarksGridDebug());
    expect(afterSameCell.gridGetRequests).toBe(afterFarCol.gridGetRequests);

    await page.evaluate(async ({ classId, markSetId, col }) => {
      await window.markbook.request("grid.updateCell", {
        classId,
        markSetId,
        row: 0,
        col: col - 1,
        value: 6.5,
        editKind: "set"
      });
    }, {
      classId: bootstrap.classId,
      markSetId: bootstrap.markSetId,
      col: prep.targetCol
    });

    const persisted = await page.evaluate(async ({ classId, markSetId, col }) => {
      const res = await window.markbook.request("grid.get", {
        classId,
        markSetId,
        rowStart: 0,
        rowCount: 1,
        colStart: col - 1,
        colCount: 1
      });
      return res.cells?.[0]?.[0] ?? null;
    }, {
      classId: bootstrap.classId,
      markSetId: bootstrap.markSetId,
      col: prep.targetCol
    });
    expect(persisted).toBeCloseTo(6.5, 5);
  } finally {
    await app.close();
  }
});
