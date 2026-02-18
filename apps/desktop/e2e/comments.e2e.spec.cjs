const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("comment set remark edits persist", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const ctx = await page.evaluate(async ({ classId, markSetId }) => {
      const sets = await window.markbook.request("comments.sets.list", { classId, markSetId });
      const setNumber = sets.sets[0]?.setNumber;
      const open = await window.markbook.request("comments.sets.open", {
        classId,
        markSetId,
        setNumber,
      });
      const studentId = open.remarksByStudent[0]?.studentId;
      return { setNumber, studentId };
    }, { classId, markSetId });
    expect(ctx.setNumber).toBeTruthy();
    expect(ctx.studentId).toBeTruthy();

    await page.getByTestId("nav-markset-setup").click();
    await page.waitForSelector('[data-testid="markset-setup-screen"]');
    await page.getByTestId("markset-setup-tab-comments").click();
    await page.waitForSelector('[data-testid="comments-panel"]');
    await page.getByTestId("comments-set-select").selectOption(String(ctx.setNumber));

    const text = `E2E remark ${Date.now()}`;
    const input = page.getByTestId(`comment-remark-input-${ctx.studentId}`);
    await input.fill(text);
    await page.getByTestId("comments-set-save-btn").click();
    await page.getByTestId("comments-reload-btn").click();

    const persisted = await page.evaluate(async ({ classId, markSetId, setNumber, studentId }) => {
      const open = await window.markbook.request("comments.sets.open", {
        classId,
        markSetId,
        setNumber,
      });
      return open.remarksByStudent.find((r) => r.studentId === studentId)?.remark ?? "";
    }, { classId, markSetId, setNumber: ctx.setNumber, studentId: ctx.studentId });

    expect(persisted).toBe(text);
  } finally {
    await app.close();
  }
});

