const { test, expect } = require("@playwright/test");
const { _electron: electron } = require("playwright");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

function mkdtemp(prefix) {
  return fs.mkdtempSync(path.join(os.tmpdir(), prefix));
}

test("marks results panel reflects calc filters (term) and matches sidecar", async () => {
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
    const res = await window.markbook.request("class.importLegacy", {
      legacyClassFolderPath,
    });
    const cls = await window.markbook.request("classes.list", {});
    const classId = cls.classes[0].id;
    const ms = await window.markbook.request("marksets.list", { classId });
    const markSetId = ms.markSets[0].id;
    const open = await window.markbook.request("markset.open", { classId, markSetId });
    const studentId = open.students[0].id;
    const sumAll = await window.markbook.request("calc.markSetSummary", {
      classId,
      markSetId,
      filters: { term: null, categoryName: null, typesMask: null },
    });
    const sAll = sumAll.perStudent.find((s) => s.studentId === studentId);
    const sumT1 = await window.markbook.request("calc.markSetSummary", {
      classId,
      markSetId,
      filters: { term: 1, categoryName: null, typesMask: null },
    });
    const sT1 = sumT1.perStudent.find((s) => s.studentId === studentId);
    return {
      className: res.name,
      classId,
      markSetId,
      studentId,
      allFinal: sAll?.finalMark ?? null,
      t1Final: sT1?.finalMark ?? null,
    };
  }, { workspacePath, legacyClassFolderPath });

  await page.getByTestId("refresh-btn").click();
  await page.getByRole("button", { name: bootstrap.className, exact: true }).click();
  await page.getByTestId("nav-marks").click();

  await page.waitForSelector('[data-testid="marks-screen"]');
  await page.waitForSelector('[data-testid="marks-results-panel"]');

  // Click near top-left of the grid to select the first student row.
  const canvas = page.getByTestId("data-grid-canvas");
  // Glide's scroller can intermittently intercept pointer events in CI; force the click.
  try {
    await canvas.click({ position: { x: 40, y: 60 }, force: true });
  } catch (e) {
    // Fallback: raw mouse click at an absolute point inside the canvas.
    const bb = await canvas.boundingBox();
    if (bb == null) throw e;
    await page.mouse.click(bb.x + 40, bb.y + 60);
  }

  const readFinal = async () => {
    const txt = await page.getByTestId("marks-results-final").innerText();
    const t = txt.trim();
    if (t === "—") return null;
    const n = Number(t);
    return Number.isFinite(n) ? n : null;
  };

  const vAll = await readFinal();
  if (bootstrap.allFinal == null) {
    expect(vAll).toBeNull();
  } else {
    expect(vAll).toBeCloseTo(bootstrap.allFinal, 1);
  }

  await page.getByTestId("marks-filter-term").selectOption("1");
  // Give the renderer time to roundtrip IPC.
  await page.waitForTimeout(200);

  const vT1 = await readFinal();
  if (bootstrap.t1Final == null) {
    expect(vT1).toBeNull();
  } else {
    expect(vT1).toBeCloseTo(bootstrap.t1Final, 1);
  }

  await app.close();
});
