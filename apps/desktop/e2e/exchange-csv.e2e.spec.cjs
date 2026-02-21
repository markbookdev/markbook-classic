const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("class exchange CSV export/import roundtrip", async () => {
  const outDir = mkdtemp("markbook-exchange-");
  const csvPath = path.join(outDir, "class-exchange.csv");
  const { app, page, repoRoot } = await launchElectronApp({
    MARKBOOK_E2E_PICK_SAVE_PATH: csvPath,
    MARKBOOK_E2E_PICK_OPEN_PATH: csvPath,
  });
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    await page.getByTestId("nav-exchange").click();
    await page.waitForSelector('[data-testid="exchange-screen"]');
    await page.getByTestId("exchange-export-browse-btn").click();
    await expect(page.getByTestId("exchange-export-path-input")).toHaveValue(csvPath);
    await page.getByTestId("exchange-export-btn").click();
    await expect(page.getByText("Exported", { exact: false })).toBeVisible();

    expect(fs.existsSync(csvPath)).toBeTruthy();
    expect(fs.statSync(csvPath).size).toBeGreaterThan(0);

    await page.getByTestId("exchange-import-browse-btn").click();
    await expect(page.getByTestId("exchange-import-path-input")).toHaveValue(csvPath);
    await page.getByTestId("exchange-preview-btn").click();
    await expect(page.getByTestId("exchange-preview-summary")).toBeVisible();
    await page.getByTestId("exchange-import-btn").click();
    await expect(page.getByText(/Imported|Applied/, { exact: false })).toBeVisible();
  } finally {
    await app.close();
  }
});
