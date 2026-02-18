const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("students membership toggle affects calc and persists", async () => {
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
    const className = cls.classes[0].name;
    const ms = await window.markbook.request("marksets.list", { classId });
    const mat1 = ms.markSets.find((m) => m.code === "MAT1") ?? ms.markSets[0];
    const markSetId = mat1.id;

    // Pick a student with a non-null final mark.
    const sum = await window.markbook.request("calc.markSetSummary", { classId, markSetId });
    const row = sum.perStudent.find((s) => s.finalMark != null);
    if (!row) throw new Error("expected at least one student with a final mark");

    const open = await window.markbook.request("markset.open", { classId, markSetId });
    const student = open.students.find((s) => s.id === row.studentId);
    if (!student) throw new Error("student not found in markset.open");

    return {
      classId,
      className,
      markSetId,
      studentId: row.studentId,
      studentSortOrder: student.sortOrder,
    };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();
  await page.getByTestId(`class-btn-${bootstrap.classId}`).click();
  await page.getByTestId(`markset-btn-${bootstrap.markSetId}`).click();

  // Toggle membership off in Students screen.
  await page.getByTestId("nav-students").click();
  await page.waitForSelector('[data-testid="students-screen"]');
  await page.getByTestId("students-membership-tab").click();
  await page.waitForSelector('[data-testid="students-membership-table-wrap"]');

  // Bulk disable for this mark set (fast parity workflow), then verify the target student is unchecked.
  await page.getByTestId(`membership-disable-all-${bootstrap.markSetId}`).click();

  const cell = page.getByTestId(`student-membership-cell-${bootstrap.studentId}-${bootstrap.markSetId}`);
  await expect(cell).not.toBeChecked();

  // Navigate to Marks and select the target student row, then verify final mark is blank.
  await page.getByTestId("nav-marks").click();
  await page.waitForSelector('[data-testid="marks-screen"]');
  await page.waitForSelector('[data-testid="marks-results-panel"]');

  // Click student row cell (col 0, row=sortOrder) to drive results panel.
  const selectedViaHarness = await page.evaluate(({ row }) => {
    return window.__markbookTest?.openMarksCellEditor?.(1, row) ?? false;
  }, { row: bootstrap.studentSortOrder });
  if (!selectedViaHarness) {
    const bounds = await page.evaluate(({ col, row }) => {
      return window.__markbookTest?.getMarksCellBounds?.(col, row) ?? null;
    }, { col: 0, row: bootstrap.studentSortOrder });
    expect(bounds).not.toBeNull();
    const canvas = page.getByTestId("data-grid-canvas");
    const bb = await canvas.boundingBox();
    expect(bb).not.toBeNull();
    // bounds are relative to the grid; offset into canvas bounds.
    await page.mouse.click(bb.x + bounds.x + bounds.width / 2, bb.y + bounds.y + bounds.height / 2);
  }

  const finalTxt = (await page.getByTestId("marks-results-final").innerText()).trim();
  expect(finalTxt).toBe("—");

  // Refresh and ensure membership persisted (still blank final mark).
  await page.getByTestId("refresh-btn").click();
  await page.getByTestId("nav-marks").click();
  await page.waitForSelector('[data-testid="marks-screen"]');
  const finalTxt2 = (await page.getByTestId("marks-results-final").innerText()).trim();
  expect(finalTxt2).toBe("—");

  // Cleanup: bulk enable so other tests/users aren't surprised when reusing this workspace.
  await page.getByTestId("nav-students").click();
  await page.waitForSelector('[data-testid="students-screen"]');
  await page.getByTestId("students-membership-tab").click();
  await page.waitForSelector('[data-testid="students-membership-table-wrap"]');
  await page.getByTestId(`membership-enable-all-${bootstrap.markSetId}`).click();
  await expect(cell).toBeChecked();

  await app.close();
});
