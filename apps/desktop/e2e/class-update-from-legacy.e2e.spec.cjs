const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("dashboard legacy preview/update runs and preserves membership overrides", async () => {
  test.setTimeout(90_000);
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(
      page,
      repoRoot,
      "fixtures/legacy/Sample25/MB8D25"
    );

    const setup = await page.evaluate(async ({ classId }) => {
      const marksets = await window.markbook.request("marksets.list", { classId });
      const mat1 = marksets.markSets.find((m) => m.code === "MAT1") ?? marksets.markSets[0];
      const membership = await window.markbook.request("students.membership.get", { classId });
      const student = membership.students.find((s) => s.active) ?? membership.students[0];
      await window.markbook.request("students.membership.set", {
        classId,
        studentId: student.id,
        markSetId: mat1.id,
        enabled: false
      });
      return { classId, markSetId: mat1.id, studentId: student.id };
    }, { classId: imported.classId });

    await page.getByTestId("nav-dashboard").click();
    await expect(page.getByTestId("class-legacy-preview-btn")).toBeVisible();
    await expect(page.getByTestId("class-update-from-legacy-btn")).toBeVisible();

    const legacyFolder = `${repoRoot}/fixtures/legacy/Sample25/MB8D25`;
    const preview = await page.evaluate(async ({ classId, legacyFolder }) => {
      return await window.markbook.request("classes.legacyPreview", {
        classId,
        legacyClassFolderPath: legacyFolder
      });
    }, { classId: setup.classId, legacyFolder });
    expect(preview.students?.matched ?? 0).toBeGreaterThan(0);

    await page.evaluate(async ({ classId, legacyFolder }) => {
      await window.markbook.request("classes.updateFromLegacy", {
        classId,
        legacyClassFolderPath: legacyFolder,
        mode: "upsert_preserve",
        collisionPolicy: "merge_existing",
        preserveLocalValidity: true
      });
    }, { classId: setup.classId, legacyFolder });

    const verify = await page.evaluate(async ({ classId, markSetId, studentId }) => {
      const membership = await window.markbook.request("students.membership.get", { classId });
      const markset = membership.markSets.find((m) => m.id === markSetId);
      const student = membership.students.find((s) => s.id === studentId);
      const idx = Number(markset?.sortOrder ?? 0);
      const bit = String(student?.mask ?? "")[idx] ?? "1";
      const meta = await window.markbook.request("classes.meta.get", { classId });
      return {
        bit,
        hasLastImportedAt: Boolean(meta?.meta?.lastImportedAt),
      };
    }, setup);

    expect(verify.bit).toBe("0");
    expect(verify.hasLastImportedAt).toBeTruthy();
  } finally {
    await app.close();
  }
});
