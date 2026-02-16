const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const { _electron: electron } = require("playwright");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

async function launchElectronApp() {
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const appDir = path.join(repoRoot, "apps", "desktop");
  const electronExecutable = require("electron");
  const mainPath = path.join(appDir, "electron", "main.js");

  const userDataDir = mkdtemp("markbook-userdata-");

  const app = await electron.launch({
    executablePath: electronExecutable,
    args: [mainPath],
    env: {
      ...process.env,
      VITE_DEV_SERVER_URL: "",
      MARKBOOK_USER_DATA_DIR: userDataDir,
    },
  });

  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForSelector('[data-testid="app-shell"]');

  return { app, page, repoRoot, userDataDir };
}

async function importLegacyFixture(page, repoRoot, fixtureRelPath) {
  const legacyClassFolderPath = path.join(repoRoot, fixtureRelPath);
  const workspacePath = mkdtemp("markbook-workspace-");

  const res = await page.evaluate(async (payload) => {
    const { workspacePath, legacyClassFolderPath } = payload;
    await window.markbook.request("workspace.select", { path: workspacePath });
    const importRes = await window.markbook.request("class.importLegacy", {
      legacyClassFolderPath,
    });
    const cls = await window.markbook.request("classes.list", {});
    const classId = cls.classes.find((c) => c.id === importRes.classId)?.id ?? cls.classes[0]?.id;
    const ms = await window.markbook.request("marksets.list", { classId });
    const markSetId = ms.markSets[0]?.id ?? null;
    return {
      workspacePath,
      classId,
      className: importRes.name,
      markSetId,
    };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();
  await page.getByRole("button", { name: res.className, exact: true }).click();
  if (res.markSetId) {
    // Wait for mark sets to load for the selected class.
    await page.waitForSelector(`[data-testid="markset-btn-${res.markSetId}"]`);
    await page.getByTestId(`markset-btn-${res.markSetId}`).click();
  }

  return res;
}

module.exports = {
  mkdtemp,
  launchElectronApp,
  importLegacyFixture,
};
