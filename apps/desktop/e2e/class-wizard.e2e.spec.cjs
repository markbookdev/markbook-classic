const { test, expect } = require("@playwright/test");
const { launchElectronApp, mkdtemp } = require("./_helpers.cjs");

test("class wizard creates class and persists class meta", async () => {
  test.setTimeout(60_000);
  const { app, page } = await launchElectronApp();
  const workspacePath = mkdtemp("markbook-class-wizard-e2e-");
  try {
    await page.evaluate(async ({ workspacePath }) => {
      await window.markbook.request("workspace.select", { path: workspacePath });
    }, { workspacePath });

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId("dashboard-open-class-wizard-btn").click();
    await expect(page.getByTestId("class-wizard-screen")).toBeVisible();

    const created = await page.evaluate(async () => {
      return await window.markbook.request("classes.createFromWizard", {
        name: "Wizard Class 8D",
        classCode: "WIZ8D",
        schoolYear: "2025/2026",
        schoolName: "Asylum School",
        teacherName: "Teacher X",
        calcMethodDefault: 2,
        weightMethodDefault: 1,
        schoolYearStartMonth: 9
      });
    });
    expect(created.classId).toBeTruthy();

    let classId = null;
    for (let i = 0; i < 30; i += 1) {
      const cls = await page.evaluate(async () => {
        const classes = await window.markbook.request("classes.list", {});
        return classes.classes.find((c) => c.name === "Wizard Class 8D") ?? null;
      });
      if (cls?.id) {
        classId = cls.id;
        break;
      }
      await page.waitForTimeout(200);
    }
    expect(classId).toBeTruthy();

    const meta = await page.evaluate(async ({ classId }) => {
      return await window.markbook.request("classes.meta.get", { classId });
    }, { classId });
    expect(meta).not.toBeNull();
    expect(meta.meta.classCode).toBe("WIZ8D");
    expect(meta.meta.createdFromWizard).toBeTruthy();
  } finally {
    await app.close();
  }
});
