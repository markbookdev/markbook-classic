const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("attendance month matrix edits persist", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );

    const student = await page.evaluate(async ({ classId }) => {
      const res = await window.markbook.request("students.list", { classId });
      return res.students[0];
    }, { classId });
    expect(student?.id).toBeTruthy();

    await page.getByTestId("nav-attendance").click();
    await page.waitForSelector('[data-testid="attendance-screen"]');
    await page.getByTestId("attendance-month-select").selectOption("9");

    const cell = page.getByTestId(`attendance-student-cell-${student.id}-1`);
    await cell.fill("A");
    await cell.press("Tab");
    await page.getByTestId("attendance-reload-btn").click();

    const loaded = await page.evaluate(async ({ classId, studentId }) => {
      const open = await window.markbook.request("attendance.monthOpen", {
        classId,
        month: "9",
      });
      const row = open.rows.find((r) => r.studentId === studentId);
      return row?.dayCodes?.[0] ?? "";
    }, { classId, studentId: student.id });

    expect(loaded).toBe("A");
  } finally {
    await app.close();
  }
});

