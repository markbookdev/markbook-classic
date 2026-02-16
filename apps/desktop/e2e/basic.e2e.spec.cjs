const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("legacy import -> marks grid edit persists (via sidecar)", async () => {
  const repoRoot = path.join(__dirname, "..", "..", "..");
  const appDir = path.join(repoRoot, "apps", "desktop");
  const legacyClassFolderPath = path.join(
    repoRoot,
    "fixtures",
    "legacy",
    "Sample25",
    "MB8D25"
  );

  const userDataDir = mkdtemp("markbook-userdata-");
  const workspacePath = mkdtemp("markbook-workspace-");

  const electronExecutable = require("electron");
  const mainPath = path.join(appDir, "electron", "main.js");

  const app = await electron.launch({
    executablePath: electronExecutable,
    // Launch the app's main process entrypoint directly (more reliable than passing the app dir).
    args: [mainPath],
    env: {
      ...process.env,
      // Ensure we load the built renderer, not the dev server.
      VITE_DEV_SERVER_URL: "",
      // Isolate prefs, etc.
      MARKBOOK_USER_DATA_DIR: userDataDir,
    },
  });

  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForSelector('[data-testid="app-shell"]');

  // Import fixture via sidecar directly (avoid OS file dialogs).
  const { className } = await page.evaluate(async (payload) => {
    const { workspacePath, legacyClassFolderPath } = payload;
    await window.markbook.request("workspace.select", { path: workspacePath });
    const res = await window.markbook.request("class.importLegacy", {
      legacyClassFolderPath,
    });
    return { className: res.name };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();

  // Select the imported class and a mark set, then open Marks.
  await page.getByRole("button", { name: className, exact: true }).click();
  await page.getByTestId("nav-marks").click();

  await page.waitForSelector('[data-testid="marks-screen"]');

  // Wait until test harness can open the custom marks editor deterministically.
  await page.waitForFunction(() => {
    const w = window;
    return typeof w.__markbookTest?.openMarksCellEditor === "function";
  });

  const input = page.getByTestId("mark-grid-editor-input");
  const opened = await page.evaluate(() => {
    const w = window;
    return Boolean(w.__markbookTest?.openMarksCellEditor?.(1, 0));
  });
  expect(opened).toBeTruthy();
  await expect(input).toBeVisible({ timeout: 5_000 });
  await input.fill("6.5");
  await input.press("Enter");

  // Verify persistence via sidecar readback.
  const v = await page.evaluate(async ({ workspacePath }) => {
    // If the app auto-opened the workspace already, this is a no-op; safe either way.
    await window.markbook.request("workspace.select", { path: workspacePath });
    const cls = await window.markbook.request("classes.list", {});
    const classId = cls.classes[0].id;
    const ms = await window.markbook.request("marksets.list", { classId });
    const markSetId = ms.markSets[0].id;
    const grid = await window.markbook.request("grid.get", {
      classId,
      markSetId,
      rowStart: 0,
      rowCount: 1,
      colStart: 0,
      colCount: 1,
    });
    return grid.cells[0][0];
  }, { workspacePath });

  expect(v).toBeCloseTo(6.5, 5);

  await app.close();
});
