const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("backup export/import restores class data", async () => {
  const outDir = mkdtemp("markbook-backup-");
  const bundlePath = path.join(outDir, "workspace-backup.mbcbackup.zip");
  const { app, page, repoRoot } = await launchElectronApp({
    MARKBOOK_E2E_PICK_SAVE_PATH: bundlePath,
    MARKBOOK_E2E_PICK_OPEN_PATH: bundlePath,
  });
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    await page.getByTestId("nav-backup").click();
    await page.waitForSelector('[data-testid="backup-screen"]');
    await page.getByTestId("backup-export-browse-btn").click();
    await expect(page.getByTestId("backup-export-path-input")).toHaveValue(bundlePath);
    await page.getByTestId("backup-export-btn").click();
    await expect(page.getByText("Exported backup", { exact: false })).toBeVisible();

    expect(fs.existsSync(bundlePath)).toBeTruthy();
    expect(fs.statSync(bundlePath).size).toBeGreaterThan(0);

    await page.evaluate(async ({ classId }) => {
      await window.markbook.request("classes.delete", { classId });
    }, { classId });

    const afterDeleteCount = await page.evaluate(async () => {
      const cls = await window.markbook.request("classes.list", {});
      return cls.classes.length;
    });
    expect(afterDeleteCount).toBe(0);

    await page.getByTestId("backup-import-browse-btn").click();
    await expect(page.getByTestId("backup-import-path-input")).toHaveValue(bundlePath);
    page.on("dialog", (d) => d.accept());
    await page.getByTestId("backup-import-btn").click();

    const restoredCount = await page.evaluate(async () => {
      const cls = await window.markbook.request("classes.list", {});
      return cls.classes.length;
    });
    expect(restoredCount).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
