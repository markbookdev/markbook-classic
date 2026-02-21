const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("setup admin settings persist and reload", async () => {
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const appDir = path.join(repoRoot, "apps", "desktop");
  const legacyClassFolderPath = path.join(repoRoot, "fixtures", "legacy", "Sample25", "MB8D25");

  const userDataDir = mkdtemp("markbook-userdata-");
  const workspacePath = mkdtemp("markbook-workspace-");

  const electronExecutable = require("electron");
  const mainPath = path.join(appDir, "electron", "main.js");

  const app = await electron.launch({
    executablePath: electronExecutable,
    args: [mainPath],
    env: {
      ...process.env,
      VITE_DEV_SERVER_URL: "",
      MARKBOOK_USER_DATA_DIR: userDataDir
    }
  });

  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForSelector('[data-testid="app-shell"]');

  const bootstrap = await page.evaluate(async ({ workspacePath, legacyClassFolderPath }) => {
    await window.markbook.request("workspace.select", { path: workspacePath });
    await window.markbook.request("class.importLegacy", { legacyClassFolderPath });
    const cls = await window.markbook.request("classes.list", {});
    return { classId: cls.classes[0].id };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();
  await page.getByTestId(`class-btn-${bootstrap.classId}`).click();
  await page.getByTestId("nav-setup-admin").click();
  await page.waitForSelector('[data-testid="setup-admin-screen"]');

  await page.getByTestId("setup-analysis-scope").selectOption("active");
  await page.getByTestId("setup-printer-font-scale").fill("115");
  await page.getByTestId("setup-comments-policy").selectOption("append");
  await page.getByTestId("setup-integrations-default-profile").selectOption("sis_marks_v1");
  await page.getByTestId("setup-integrations-match-mode").selectOption("name_only");
  await page.getByTestId("setup-integrations-collision-policy").selectOption("append_new");
  await page.getByTestId("setup-integrations-admin-policy").selectOption("source_if_longer");
  await page.getByTestId("setup-reports-default-analytics-scope").selectOption("active");
  await page.getByTestId("setup-save-all").click();

  // Navigate away and back to ensure reload path works.
  await page.getByTestId("nav-marks").click();
  await page.waitForSelector('[data-testid="marks-screen"]');
  await page.getByTestId("nav-setup-admin").click();
  await page.waitForSelector('[data-testid="setup-admin-screen"]');

  const persisted = await page.evaluate(async () => {
    return await window.markbook.request("setup.get", {});
  });
  expect(persisted.analysis.defaultStudentScope).toBe("active");
  expect(persisted.printer.fontScale).toBe(115);
  expect(persisted.comments.defaultTransferPolicy).toBe("append");
  expect(persisted.integrations.defaultSisProfile).toBe("sis_marks_v1");
  expect(persisted.integrations.defaultMatchMode).toBe("name_only");
  expect(persisted.integrations.defaultCollisionPolicy).toBe("append_new");
  expect(persisted.integrations.adminTransferDefaultPolicy).toBe("source_if_longer");
  expect(persisted.reports.defaultAnalyticsScope).toBe("active");

  await app.close();
});
