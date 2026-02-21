const { test, expect } = require("@playwright/test");
const { launchElectronApp, importLegacyFixture } = require("./_helpers.cjs");

test("legacy menu groups are discoverable and route to implemented screens", async () => {
  const { app, page, repoRoot } = await launchElectronApp();
  try {
    const imported = await importLegacyFixture(page, repoRoot, "fixtures/legacy/Sample25/MB8D25");
    await page.getByTestId(`class-btn-${imported.classId}`).click();

    const groups = page.getByTestId("legacy-menu-groups");
    await expect(groups).toBeVisible();
    for (const groupId of [
      "menu-group-file",
      "menu-group-class",
      "menu-group-marksets",
      "menu-group-workingon",
      "menu-group-reports",
      "menu-group-comments",
      "menu-group-setup",
      "menu-group-integrations",
      "menu-group-planner"
    ]) {
      await expect(groups.getByTestId(groupId)).toBeVisible();
    }

    for (const groupId of [
      "menu-group-file",
      "menu-group-class",
      "menu-group-marksets",
      "menu-group-workingon",
      "menu-group-reports",
      "menu-group-comments",
      "menu-group-setup",
      "menu-group-integrations",
      "menu-group-planner"
    ]) {
      await groups.getByTestId(groupId).locator("summary").click();
    }
    await groups.locator('summary:has-text("Help")').click();

    const expectedLabels = [
      "Make a New Class",
      "Edit Class Profile",
      "Open a Class",
      "BackUp",
      "Exports",
      "Select Printer",
      "Class List",
      "Attendance",
      "Seating",
      "Student Notes",
      "Email Class List",
      "Make a New Mark Set",
      "Open a Mark Set",
      "Edit Heading and Categories",
      "Undelete a Mark Set",
      "Entry Heading",
      "Edit Marks",
      "Display/Print",
      "Clone Entry",
      "Mark Set Reports",
      "Class Analytics",
      "Student Analytics",
      "Combined Analytics",
      "Remarks in Marks",
      "Comment Sets",
      "Comment Banks",
      "Transfer Mode",
      "Analysis/Report Options",
      "Calculation Setup",
      "Planner Setup",
      "Comments Setup",
      "Printer Options",
      "Password + Email Setup",
      "Class Exchange",
      "SIS",
      "Admin Transfer",
      "Units + Lessons",
      "Course Description",
      "Planner Reports",
      "Legacy Actions Map"
    ];
    for (const label of expectedLabels) {
      await expect(groups.getByRole("button", { name: label, exact: true })).toBeVisible();
    }

    await groups.getByRole("button", { name: "Combined Analytics", exact: true }).click();
    await expect(page.getByTestId("combined-analytics-screen")).toBeVisible();

    await groups.getByRole("button", { name: "Remarks in Marks", exact: true }).click();
    await expect(page.getByTestId("marks-screen")).toBeVisible();

    const pendingPrinter = groups.getByRole("button", { name: "Select Printer", exact: true });
    await expect(pendingPrinter).toBeDisabled();
    await expect(pendingPrinter).toHaveAttribute("title", "Not implemented yet");

    const pendingEmail = groups.getByRole("button", { name: "Email Class List", exact: true });
    await expect(pendingEmail).toBeDisabled();
    await expect(pendingEmail).toHaveAttribute("title", "Not implemented yet");
  } finally {
    await app.close();
  }
});
