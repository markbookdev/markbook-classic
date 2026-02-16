const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("class exchange CSV export/import roundtrip", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    const outDir = mkdtemp("markbook-exchange-");
    const csvPath = path.join(outDir, "class-exchange.csv");

    const exported = await page.evaluate(async ({ classId, csvPath }) => {
      return await window.markbook.request("exchange.exportClassCsv", {
        classId,
        outPath: csvPath,
      });
    }, { classId, csvPath });

    expect(exported.rowsExported).toBeGreaterThan(0);
    expect(fs.existsSync(csvPath)).toBeTruthy();

    const imported = await page.evaluate(async ({ classId, csvPath }) => {
      return await window.markbook.request("exchange.importClassCsv", {
        classId,
        inPath: csvPath,
        mode: "upsert",
      });
    }, { classId, csvPath });

    expect(imported.updated).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
