const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const path = require("node:path");
const fs = require("node:fs");

test("integrations SIS preview/apply/export workflow", async () => {
  const outDir = mkdtemp("markbook-integrations-sis-");
  const sisImportPath = path.join(outDir, "sis-import.csv");
  const sisRosterPath = path.join(outDir, "sis-roster.csv");
  const sisMarksPath = path.join(outDir, "sis-marks.csv");
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(
      page,
      repoRoot,
      path.join("fixtures", "legacy", "Sample25", "MB8D25")
    );
    expect(classId).toBeTruthy();

    const students = await page.evaluate(async ({ classId }) => {
      return await window.markbook.request("students.list", { classId });
    }, { classId });
    const first = students.students[0];
    const sisCsv = [
      "student_no,last_name,first_name,active,birth_date",
      `${first.studentNo || ""},${first.lastName},${first.firstName},1,2008-01-01`,
      "990001,Integration,Tester,1,2009-03-10"
    ].join("\n");
    fs.writeFileSync(sisImportPath, `${sisCsv}\n`, "utf8");

    await page.getByTestId("nav-exchange").click();
    await page.waitForSelector('[data-testid="exchange-screen"]');
    await page.getByTestId("integrations-sis-tab").click();
    await page.getByTestId("integrations-sis-path-input").fill(sisImportPath);
    await page.getByTestId("integrations-sis-preview-btn").click();
    await expect(page.getByText(/SIS preview:/)).toBeVisible();

    await page.getByTestId("integrations-sis-apply-btn").click();
    await expect(page.getByText(/SIS apply complete:/)).toBeVisible();

    const hasImported = await page.evaluate(async ({ classId }) => {
      const res = await window.markbook.request("students.list", { classId });
      return res.students.some((s) => s.lastName === "Integration" && s.firstName === "Tester");
    }, { classId });
    expect(hasImported).toBeTruthy();

    await page.getByPlaceholder("/absolute/path/to/sis-roster.csv").fill(sisRosterPath);
    await page.getByTestId("integrations-sis-export-roster-btn").click();
    await expect(page.getByText(/Exported SIS roster:/)).toBeVisible();
    expect(fs.existsSync(sisRosterPath)).toBeTruthy();

    await page.getByPlaceholder("/absolute/path/to/sis-marks.csv").fill(sisMarksPath);
    await page.getByTestId("integrations-sis-export-marks-btn").click();
    await expect(page.getByText(/Exported SIS marks:/)).toBeVisible();
    expect(fs.existsSync(sisMarksPath)).toBeTruthy();
    expect(fs.readFileSync(sisMarksPath, "utf8")).toContain("assessment_idx");
  } finally {
    await app.close();
  }
});
