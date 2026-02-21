const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture, mkdtemp } = require("./_helpers.cjs");
const fs = require("node:fs");
const path = require("node:path");

async function waitForPdf(outPath) {
  for (let i = 0; i < 50; i += 1) {
    if (fs.existsSync(outPath) && fs.statSync(outPath).size > 0) return;
    // eslint-disable-next-line no-await-in-loop
    await new Promise((r) => setTimeout(r, 100));
  }
}

test("planner and course-description report exports write PDFs", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const { classId } = await importLegacyFixture(page, repoRoot, path.join("fixtures", "legacy", "Sample25", "MB8D25"));

    const ids = await page.evaluate(async ({ classId }) => {
      const unit = await window.markbook.request("planner.units.create", {
        classId,
        input: { title: "Report Unit" }
      });
      const unitId = unit.unitId;
      const lesson = await window.markbook.request("planner.lessons.create", {
        classId,
        input: { title: "Report Lesson", unitId, durationMinutes: 70 }
      });
      return { unitId, lessonId: lesson.lessonId };
    }, { classId });

    const outDir = mkdtemp("markbook-planner-pdf-");
    const plannerUnitPath = path.join(outDir, "planner-unit.pdf");
    const plannerLessonPath = path.join(outDir, "planner-lesson.pdf");
    const courseDescPath = path.join(outDir, "course-description.pdf");
    const timeMgmtPath = path.join(outDir, "time-management.pdf");

    await page.evaluate(async ({ classId, unitId, lessonId, plannerUnitPath, plannerLessonPath, courseDescPath, timeMgmtPath }) => {
      await window.__markbookTest.exportPlannerUnitPdfToPath(classId, unitId, plannerUnitPath);
      await window.__markbookTest.exportPlannerLessonPdfToPath(classId, lessonId, plannerLessonPath);
      await window.__markbookTest.exportCourseDescriptionPdfToPath(classId, courseDescPath);
      await window.__markbookTest.exportTimeManagementPdfToPath(classId, timeMgmtPath);
    }, {
      classId,
      unitId: ids.unitId,
      lessonId: ids.lessonId,
      plannerUnitPath,
      plannerLessonPath,
      courseDescPath,
      timeMgmtPath
    });

    await waitForPdf(plannerUnitPath);
    await waitForPdf(plannerLessonPath);
    await waitForPdf(courseDescPath);
    await waitForPdf(timeMgmtPath);

    expect(fs.existsSync(plannerUnitPath)).toBeTruthy();
    expect(fs.statSync(plannerUnitPath).size).toBeGreaterThan(1000);
    expect(fs.existsSync(plannerLessonPath)).toBeTruthy();
    expect(fs.statSync(plannerLessonPath).size).toBeGreaterThan(1000);
    expect(fs.existsSync(courseDescPath)).toBeTruthy();
    expect(fs.statSync(courseDescPath).size).toBeGreaterThan(1000);
    expect(fs.existsSync(timeMgmtPath)).toBeTruthy();
    expect(fs.statSync(timeMgmtPath).size).toBeGreaterThan(1000);
  } finally {
    await app.close();
  }
});
