const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("backup export/import restores class data", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, workspacePath } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    const outDir = mkdtemp("markbook-backup-");
    const bundlePath = path.join(outDir, "workspace-backup.sqlite3");

    await page.evaluate(async ({ workspacePath, bundlePath }) => {
      await window.markbook.request("backup.exportWorkspaceBundle", {
        workspacePath,
        outPath: bundlePath,
      });
    }, { workspacePath, bundlePath });

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

    await page.evaluate(async ({ workspacePath, bundlePath }) => {
      await window.markbook.request("backup.importWorkspaceBundle", {
        inPath: bundlePath,
        workspacePath,
      });
    }, { workspacePath, bundlePath });

    const restoredCount = await page.evaluate(async () => {
      const cls = await window.markbook.request("classes.list", {});
      return cls.classes.length;
    });
    expect(restoredCount).toBeGreaterThan(0);
  } finally {
    await app.close();
  }
});
