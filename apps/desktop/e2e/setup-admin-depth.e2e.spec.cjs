const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("setup admin depth fields persist and reload", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");

    await page.getByTestId(`class-btn-${imported.classId}`).click();
    await page.getByTestId("nav-setup-admin").click();
    await page.waitForSelector('[data-testid="setup-admin-screen"]');

    await page.evaluate(async () => {
      await window.markbook.request("setup.update", {
        section: "comments",
        patch: {
          defaultSetNumber: 3,
          defaultAppendSeparator: "::",
          enforceMaxCharsByDefault: false
        }
      });
      await window.markbook.request("setup.update", {
        section: "printer",
        patch: {
          defaultPaperSize: "a4",
          defaultOrientation: "landscape"
        }
      });
      await window.markbook.request("setup.update", {
        section: "security",
        patch: {
          requireWorkspacePassword: true
        }
      });
      await window.markbook.request("setup.update", {
        section: "reports",
        patch: {
          repeatHeadersByDefault: false,
          defaultPageMargins: {
            topMm: 10,
            rightMm: 11,
            bottomMm: 12,
            leftMm: 13
          }
        }
      });
    });

    const persisted = await page.evaluate(async () => window.markbook.request("setup.get", {}));
    expect(persisted.comments.defaultSetNumber).toBe(3);
    expect(persisted.comments.defaultAppendSeparator).toBe("::");
    expect(persisted.comments.enforceMaxCharsByDefault).toBe(false);
    expect(persisted.printer.defaultPaperSize).toBe("a4");
    expect(persisted.printer.defaultOrientation).toBe("landscape");
    expect(persisted.security.requireWorkspacePassword).toBe(true);
    expect(persisted.reports.repeatHeadersByDefault).toBe(false);
    expect(persisted.reports.defaultPageMargins.topMm).toBe(10);
    expect(persisted.reports.defaultPageMargins.rightMm).toBe(11);
    expect(persisted.reports.defaultPageMargins.bottomMm).toBe(12);
    expect(persisted.reports.defaultPageMargins.leftMm).toBe(13);
  } finally {
    await app.close();
  }
});
