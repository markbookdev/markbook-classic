const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("comments transfer mode preview/apply copies remarks with selected policy", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId, markSetId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();
    expect(markSetId).toBeTruthy();

    const seed = await page.evaluate(async ({ classId, markSetId }) => {
      const studentsRes = await window.markbook.request("students.list", { classId });
      const students = studentsRes.students || [];
      const sourceStudentId = students[0]?.id;
      const targetStudentId = students[1]?.id;
      if (!sourceStudentId || !targetStudentId) {
        throw new Error("expected at least two students");
      }
      await window.markbook.request("comments.sets.upsert", {
        classId,
        markSetId,
        setNumber: 1,
        title: "Source",
        fitMode: 0,
        fitFontSize: 9,
        fitWidth: 83,
        fitLines: 12,
        fitSubj: "",
        maxChars: 100,
        isDefault: true,
        remarksByStudent: [
          { studentId: sourceStudentId, remark: "Source Baseline" },
          { studentId: targetStudentId, remark: "Transfer Remark Alpha" }
        ]
      });
      await window.markbook.request("comments.sets.upsert", {
        classId,
        markSetId,
        setNumber: 2,
        title: "Target",
        fitMode: 0,
        fitFontSize: 9,
        fitWidth: 83,
        fitLines: 12,
        fitSubj: "",
        maxChars: 100,
        isDefault: false,
        remarksByStudent: [
          { studentId: sourceStudentId, remark: "" },
          { studentId: targetStudentId, remark: "" }
        ]
      });
      return { sourceStudentId, targetStudentId };
    }, { classId, markSetId });

    await page.getByTestId("nav-markset-setup").click();
    await page.waitForSelector('[data-testid="markset-setup-screen"]');
    await page.getByTestId("markset-setup-tab-comments").click();
    await page.waitForSelector('[data-testid="comments-panel"]');
    await page.getByTestId("comments-set-select").selectOption("2");

    await page.getByTestId("comments-transfer-open-btn").click();
    await page.getByTestId("comments-transfer-preview-btn").click();
    await page.waitForSelector('[data-testid="comments-transfer-preview-summary"]');
    await page.getByTestId("comments-transfer-policy").selectOption("replace");
    await page.getByTestId("comments-transfer-apply-btn").click();
    await page.waitForTimeout(200);

    const transferred = await page.evaluate(async ({ classId, markSetId, targetStudentId }) => {
      const open = await window.markbook.request("comments.sets.open", {
        classId,
        markSetId,
        setNumber: 2
      });
      return (
        open.remarksByStudent.find((r) => r.studentId === targetStudentId)?.remark ?? ""
      );
    }, { classId, markSetId, targetStudentId: seed.targetStudentId });

    expect(transferred).toBe("Transfer Remark Alpha");
  } finally {
    await app.close();
  }
});
