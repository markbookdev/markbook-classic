const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("integrations admin transfer export/preview/apply workflow", async () => {
  const outDir = mkdtemp("markbook-integrations-admin-");
  const packagePath = path.join(outDir, "admin-transfer.zip");
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId: sourceClassId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(sourceClassId).toBeTruthy();

    await page.getByTestId("nav-exchange").click();
    await page.waitForSelector('[data-testid="exchange-screen"]');
    const firstAdminTab = page.getByTestId("integrations-admin-tab");
    if (!(await firstAdminTab.isDisabled())) {
      await firstAdminTab.click();
    }
    await page.locator('input[placeholder="/absolute/path/to/admin-transfer.zip"]').nth(1).fill(packagePath);
    await page.getByTestId("integrations-admin-export-btn").click();
    await expect(page.getByText(/Exported admin package/)).toBeVisible();
    expect(fs.existsSync(packagePath)).toBeTruthy();

    const created = await page.evaluate(async () => {
      const res = await window.markbook.request("classes.create", {
        name: "Admin Transfer Target E2E"
      });
      return res.classId;
    });

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId(`class-btn-${created}`).click();
    await page.getByTestId("nav-exchange").click();
    await page.waitForSelector('[data-testid="exchange-screen"]');
    const secondAdminTab = page.getByTestId("integrations-admin-tab");
    if (!(await secondAdminTab.isDisabled())) {
      await secondAdminTab.click();
    }
    await page.locator('input[placeholder="/absolute/path/to/admin-transfer.zip"]').nth(0).fill(packagePath);
    await page.getByTestId("integrations-admin-preview-btn").click();
    await expect(page.getByText(/Admin preview:/)).toBeVisible();

    await page.getByTestId("integrations-admin-apply-btn").click();
    await expect(page.getByText(/Admin apply complete:/)).toBeVisible();

    const targetHasMarkSets = await page.evaluate(async ({ classId }) => {
      const marksets = await window.markbook.request("marksets.list", { classId });
      return Array.isArray(marksets.markSets) && marksets.markSets.length > 0;
    }, { classId: created });
    expect(targetHasMarkSets).toBeTruthy();
  } finally {
    await app.close();
  }
});
