const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("marks screen can edit/save remarks for selected student", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const bootstrap = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(bootstrap.classId).toBeTruthy();
    expect(bootstrap.markSetId).toBeTruthy();

    const details = await page.evaluate(async ({ classId, markSetId }) => {
      const open = await window.markbook.request("markset.open", { classId, markSetId });
      const studentId = open.students[0]?.id;
      if (!studentId) throw new Error("expected at least one student");

      let sets = await window.markbook.request("comments.sets.list", { classId, markSetId });
      if (!Array.isArray(sets.sets) || sets.sets.length === 0) {
        await window.markbook.request("comments.sets.upsert", {
          classId,
          markSetId,
          title: "Auto Set",
          setNumber: 1,
          fitMode: 0,
          fitFontSize: 9,
          fitWidth: 83,
          fitLines: 12,
          fitSubj: "",
          maxChars: 100,
          isDefault: true,
          remarksByStudent: []
        });
        sets = await window.markbook.request("comments.sets.list", { classId, markSetId });
      }

      const setNumber = sets.sets[0]?.setNumber;
      if (typeof setNumber !== "number") throw new Error("expected comment set");
      return { studentId, setNumber };
    }, {
      classId: bootstrap.classId,
      markSetId: bootstrap.markSetId
    });

    await page.getByTestId("nav-marks").click();
    await page.waitForSelector('[data-testid="marks-screen"]');
    await page.waitForSelector('[data-testid="marks-results-panel"]');
    const canvas = page.getByTestId("data-grid-canvas");
    try {
      await canvas.click({ position: { x: 40, y: 60 }, force: true });
    } catch (e) {
      const bb = await canvas.boundingBox();
      if (bb == null) throw e;
      await page.mouse.click(bb.x + 40, bb.y + 60);
    }
    await page.getByTestId("marks-remark-set-select").selectOption(String(details.setNumber));

    const remarkText = `Remark ${Date.now()}`;
    await page.getByTestId("marks-remark-textarea").fill(remarkText);
    await expect(page.getByTestId("marks-remark-save-btn")).toBeEnabled();
    await page.getByTestId("marks-remark-save-btn").click();

    await expect
      .poll(
        async () =>
          await page.evaluate(async ({ classId, markSetId, setNumber, studentId }) => {
            const open = await window.markbook.request("comments.sets.open", {
              classId,
              markSetId,
              setNumber
            });
            const row = open.remarksByStudent.find((r) => r.studentId === studentId);
            return row?.remark ?? "";
          }, {
            classId: bootstrap.classId,
            markSetId: bootstrap.markSetId,
            setNumber: details.setNumber,
            studentId: details.studentId
          }),
        { timeout: 20000 }
      )
      .toBe(remarkText);

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId("nav-marks").click();
    await page.waitForSelector('[data-testid="marks-screen"]');
    await page.getByTestId("marks-remark-set-select").selectOption(String(details.setNumber));
    await expect(page.getByTestId("marks-remark-textarea")).toHaveValue(remarkText);
  } finally {
    await app.close();
  }
});
