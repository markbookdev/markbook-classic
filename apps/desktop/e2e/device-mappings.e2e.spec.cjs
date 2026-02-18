const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");
const path = require("node:path");

test("device mappings screen updates and clears mapping", async () => {
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

    await page.getByTestId("nav-device-mappings").click();
    await page.waitForSelector('[data-testid="devices-screen"]');
    await page.waitForSelector(`[data-testid=\"devices-row-${student.id}\"]`);
    await page.evaluate(async ({ classId, studentId }) => {
      await window.markbook.request("devices.update", {
        classId,
        studentId,
        deviceCode: "IPAD-999",
        rawLine: ""
      });
    }, { classId, studentId: student.id });

    const updated = await page.evaluate(async ({ classId, studentId }) => {
      const res = await window.markbook.request("devices.get", { classId, studentId });
      return res.device.deviceCode;
    }, { classId, studentId: student.id });
    expect(updated).toBe("IPAD-999");

    await page.evaluate(async ({ classId, studentId }) => {
      await window.markbook.request("devices.update", {
        classId,
        studentId,
        deviceCode: "",
        rawLine: ""
      });
    }, { classId, studentId: student.id });

    const cleared = await page.evaluate(async ({ classId, studentId }) => {
      const res = await window.markbook.request("devices.get", { classId, studentId });
      return res.device.deviceCode;
    }, { classId, studentId: student.id });
    expect(cleared).toBe("");
  } finally {
    await app.close();
  }
});
