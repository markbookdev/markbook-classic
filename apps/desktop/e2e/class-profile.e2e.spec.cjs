const { test, expect } = require("@playwright/test");
const { launchElectronApp, mkdtemp } = require("./_helpers.cjs");

test("class profile editor updates class meta and class name", async () => {
  test.setTimeout(60_000);
  const { app, page } = await launchElectronApp();
  const workspacePath = mkdtemp("markbook-class-profile-e2e-");
  try {
    const created = await page.evaluate(async ({ workspacePath }) => {
      await window.markbook.request("workspace.select", { path: workspacePath });
      const cls = await window.markbook.request("classes.create", { name: "Profile Class" });
      await window.markbook.request("classes.meta.update", {
        classId: cls.classId,
        patch: {
          classCode: "PCU8D",
          schoolYear: "2026/2027",
          schoolName: "Asylum School",
          teacherName: "Teacher Profile",
          calcMethodDefault: 0,
          weightMethodDefault: 1,
          schoolYearStartMonth: 9
        }
      });
      return cls;
    }, { workspacePath });
    expect(created.classId).toBeTruthy();

    await page.getByTestId("refresh-btn").click();
    await page.getByTestId(`class-btn-${created.classId}`).click();
    await expect(page.getByTestId("nav-class-profile")).toBeEnabled();
    await page.getByTestId("nav-class-profile").click();
    await expect(page.getByTestId("class-wizard-screen")).toBeVisible();
    await page.getByTestId("class-wizard-mode-edit").click();
    await expect(page.getByTestId("class-meta-save-btn")).toBeVisible();
    await page.getByTestId("class-wizard-name").fill("Profile Class Updated");
    await page.getByTestId("class-meta-save-btn").click();

    let meta = null;
    for (let i = 0; i < 30; i += 1) {
      meta = await page.evaluate(async ({ classId }) => {
        return await window.markbook.request("classes.meta.get", { classId });
      }, { classId: created.classId });
      if (
        meta?.meta?.classCode === "PCU8D" &&
        meta?.meta?.teacherName === "Teacher Profile"
      ) {
        break;
      }
      await page.waitForTimeout(200);
    }
    expect(meta?.meta?.classCode).toBe("PCU8D");
    expect(meta?.meta?.teacherName).toBe("Teacher Profile");

    let className = null;
    for (let i = 0; i < 30; i += 1) {
      className = await page.evaluate(async ({ classId }) => {
        const cls = await window.markbook.request("classes.list", {});
        return cls.classes.find((c) => c.id === classId)?.name ?? null;
      }, { classId: created.classId });
      if (className === "Profile Class Updated") break;
      await page.waitForTimeout(200);
    }
    expect(className).toBe("Profile Class Updated");
  } finally {
    await app.close();
  }
});
