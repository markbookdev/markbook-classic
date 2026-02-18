const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("calc settings overrides affect Mode results and can be cleared", async () => {
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
      MARKBOOK_USER_DATA_DIR: userDataDir,
    },
  });

  const page = await app.firstWindow();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForSelector('[data-testid="app-shell"]');

  const bootstrap = await page.evaluate(async (payload) => {
    const { workspacePath, legacyClassFolderPath } = payload;
    await window.markbook.request("workspace.select", { path: workspacePath });
    await window.markbook.request("class.importLegacy", { legacyClassFolderPath });
    const cls = await window.markbook.request("classes.list", {});
    const classId = cls.classes[0].id;
    const ms = await window.markbook.request("marksets.list", { classId });
    const mat1 = ms.markSets.find((m) => m.code === "MAT1") ?? ms.markSets[0];
    const markSetId = mat1.id;

    // Force Mode method for a deterministic config test.
    await window.markbook.request("markset.settings.update", {
      classId,
      markSetId,
      patch: { calcMethod: 2, weightMethod: 1 },
    });

    const sum = await window.markbook.request("calc.markSetSummary", { classId, markSetId });
    const row = sum.perStudent.find((s) => s.finalMark != null);
    if (!row) throw new Error("expected at least one student with a final mark under Mode");

    return {
      classId,
      markSetId,
      studentId: row.studentId,
    };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();
  await page.getByTestId(`class-btn-${bootstrap.classId}`).click();
  await page.getByTestId(`markset-btn-${bootstrap.markSetId}`).click();
  const fetchExpectedFinal = async () =>
    page.evaluate(async ({ classId, markSetId, studentId }) => {
      const sum = await window.markbook.request("calc.markSetSummary", { classId, markSetId });
      const row = sum.perStudent.find((s) => s.studentId === studentId);
      return row?.finalMark ?? null;
    }, {
      classId: bootstrap.classId,
      markSetId: bootstrap.markSetId,
      studentId: bootstrap.studentId,
    });

  const baselineExpected = await fetchExpectedFinal();
  expect(baselineExpected).not.toBeNull();

  // Apply an override that forces all marks into level 1 (midrange 50.0) under activeLevels=1.
  await page.getByTestId("nav-calc-settings").click();
  await page.waitForSelector('[data-testid="calc-settings-screen"]');

  await page.getByTestId("calc-settings-levels").fill("1");
  // Find the level=1 threshold input (row 1, threshold column) by using the table order.
  const rows = page.locator('[data-testid="calc-settings-screen"] table tbody tr');
  await rows.nth(1).locator("input").first().fill("0");
  await page.getByTestId("calc-settings-save").click();
  await page.waitForTimeout(200);

  const forced = await fetchExpectedFinal();
  expect(forced).not.toBeNull();
  expect(forced).toBeCloseTo(50.0, 1);

  // Clear override and ensure it reverts (not 50.0).
  page.once("dialog", (d) => d.accept());
  await page.getByTestId("calc-settings-clear").click();
  await page.waitForTimeout(200);

  const reverted = await fetchExpectedFinal();
  expect(reverted).not.toBeNull();
  expect(reverted).toBeCloseTo(baselineExpected, 1);

  // Sanity check that Marks screen still loads after settings edits.
  await page.getByTestId("nav-marks").click();
  await page.waitForSelector('[data-testid="marks-screen"]');

  await app.close();
});
