const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const fs = require("node:fs");
const path = require("node:path");

test("export mark set summary PDF writes a non-empty file", async () => {
  const { app, page, repoRoot } = await launchElectronApp();

  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const outDir = mkdtemp("markbook-pdf-summary-");
    const outPath = path.join(outDir, "markset-summary.pdf");

    await page.evaluate(async ({ classId, markSetId, outPath }) => {
      if (!window.__markbookTest?.exportMarkSetSummaryPdfToPath) {
        throw new Error("missing window.__markbookTest.exportMarkSetSummaryPdfToPath");
      }
      await window.__markbookTest.exportMarkSetSummaryPdfToPath(
        classId,
        markSetId,
        outPath
      );
    }, { classId, markSetId, outPath });

    for (let i = 0; i < 50; i++) {
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

