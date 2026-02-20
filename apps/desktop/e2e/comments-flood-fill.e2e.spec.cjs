const { test, expect } = require("@playwright/test");
const path = require("node:path");
const { importLegacyFixture, launchElectronApp } = require("./_helpers.cjs");

test("comments flood fill applies selected student remark to target rows", async () => {
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
      const targetA = students[1]?.id;
      const targetB = students[2]?.id;
      if (!sourceStudentId || !targetA || !targetB) {
        throw new Error("expected at least three students");
      }
      await window.markbook.request("comments.sets.upsert", {
        classId,
        markSetId,
        setNumber: 1,
        title: "Flood",
        fitMode: 0,
        fitFontSize: 9,
        fitWidth: 83,
        fitLines: 12,
        fitSubj: "",
        maxChars: 100,
        isDefault: true,
        remarksByStudent: [
          { studentId: sourceStudentId, remark: "Flood Fill Text" },
          { studentId: targetA, remark: "" },
          { studentId: targetB, remark: "" }
        ]
      });
      return { sourceStudentId, targetA, targetB };
    }, { classId, markSetId });

    await page.getByTestId("nav-markset-setup").click();
    await page.waitForSelector('[data-testid="markset-setup-screen"]');
    await page.getByTestId("markset-setup-tab-comments").click();
    await page.waitForSelector('[data-testid="comments-panel"]');
    await page.getByTestId("comments-set-select").selectOption("1");
    await page.getByTestId(`comment-remark-row-${seed.sourceStudentId}`).click();

    await page.getByTestId("comments-floodfill-open-btn").click();
    await page.getByTestId("comments-floodfill-apply-btn").click();
    await page.waitForTimeout(200);

    const result = await page.evaluate(async ({ classId, markSetId, targetA, targetB }) => {
      const open = await window.markbook.request("comments.sets.open", {
        classId,
        markSetId,
        setNumber: 1
      });
      const byId = new Map(open.remarksByStudent.map((r) => [r.studentId, r.remark]));
      return {
        a: byId.get(targetA) || "",
        b: byId.get(targetB) || ""
      };
    }, { classId, markSetId, targetA: seed.targetA, targetB: seed.targetB });

    expect(result.a).toBe("Flood Fill Text");
    expect(result.b).toBe("Flood Fill Text");
  } finally {
    await app.close();
  }
});

