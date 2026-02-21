export type AppScreen =
  | "dashboard"
  | "marks"
  | "class_wizard"
  | "reports"
  | "class_analytics"
  | "student_analytics"
  | "combined_analytics"
  | "legacy_actions_map"
  | "planner"
  | "course_description"
  | "students"
  | "markset_setup"
  | "attendance"
  | "notes"
  | "seating_plan"
  | "learning_skills"
  | "loaned_items"
  | "device_mappings"
  | "calc_settings"
  | "setup_admin"
  | "backup"
  | "exchange";

export type LegacyMenuGroupId =
  | "file"
  | "class"
  | "marksets"
  | "workingon"
  | "reports"
  | "comments"
  | "setup"
  | "integrations"
  | "planner"
  | "help";

export type LegacyMenuAction = {
  id: string;
  group: LegacyMenuGroupId;
  label: string;
  screenRoute: AppScreen | null;
  implemented: boolean;
  enabledReason: string;
  pendingReason: string;
  testId: string;
  requiresClass?: boolean;
};

export const LEGACY_MENU_GROUP_ORDER: LegacyMenuGroupId[] = [
  "file",
  "class",
  "marksets",
  "workingon",
  "reports",
  "comments",
  "setup",
  "integrations",
  "planner",
  "help"
];

export const LEGACY_MENU_GROUP_LABELS: Record<LegacyMenuGroupId, string> = {
  file: "File",
  class: "Class",
  marksets: "Mark Sets",
  workingon: "Working On",
  reports: "Reports",
  comments: "Comments",
  setup: "Setup",
  integrations: "Integrations",
  planner: "Planner",
  help: "Help"
};

export const LEGACY_MENU_GROUP_TEST_IDS: Partial<Record<LegacyMenuGroupId, string>> = {
  file: "menu-group-file",
  class: "menu-group-class",
  marksets: "menu-group-marksets",
  workingon: "menu-group-workingon",
  reports: "menu-group-reports",
  comments: "menu-group-comments",
  setup: "menu-group-setup",
  integrations: "menu-group-integrations",
  planner: "menu-group-planner"
};

export const LEGACY_MENU_ACTIONS: LegacyMenuAction[] = [
  {
    id: "file.new_class",
    group: "file",
    label: "Make a New Class",
    screenRoute: "class_wizard",
    implemented: true,
    enabledReason: "Starts the class wizard.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-new-class"
  },
  {
    id: "file.edit_class_profile",
    group: "file",
    label: "Edit Class Profile",
    screenRoute: "class_wizard",
    implemented: true,
    enabledReason: "Opens class profile editor for selected class.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-edit-class-profile",
    requiresClass: true
  },
  {
    id: "file.open_class",
    group: "file",
    label: "Open a Class",
    screenRoute: "dashboard",
    implemented: true,
    enabledReason: "Returns to class dashboard.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-open-class"
  },
  {
    id: "file.backup",
    group: "file",
    label: "BackUp",
    screenRoute: "backup",
    implemented: true,
    enabledReason: "Opens backup/restore workflow.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-backup"
  },
  {
    id: "file.exports",
    group: "file",
    label: "Exports",
    screenRoute: "exchange",
    implemented: true,
    enabledReason: "Opens exchange/integrations workflows.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-exports"
  },
  {
    id: "file.select_printer",
    group: "file",
    label: "Select Printer",
    screenRoute: null,
    implemented: false,
    enabledReason: "",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-file-select-printer"
  },
  {
    id: "class.class_list",
    group: "class",
    label: "Class List",
    screenRoute: "students",
    implemented: true,
    enabledReason: "Opens student/class list.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-class-class-list",
    requiresClass: true
  },
  {
    id: "class.attendance",
    group: "class",
    label: "Attendance",
    screenRoute: "attendance",
    implemented: true,
    enabledReason: "Opens attendance workflows.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-class-attendance",
    requiresClass: true
  },
  {
    id: "class.seating",
    group: "class",
    label: "Seating",
    screenRoute: "seating_plan",
    implemented: true,
    enabledReason: "Opens seating plans.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-class-seating",
    requiresClass: true
  },
  {
    id: "class.student_notes",
    group: "class",
    label: "Student Notes",
    screenRoute: "notes",
    implemented: true,
    enabledReason: "Opens student notes.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-class-student-notes",
    requiresClass: true
  },
  {
    id: "class.email_class_list",
    group: "class",
    label: "Email Class List",
    screenRoute: null,
    implemented: false,
    enabledReason: "",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-class-email-class-list"
  },
  {
    id: "marksets.new_mark_set",
    group: "marksets",
    label: "Make a New Mark Set",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Opens mark set manager/create flow.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-marksets-new-markset",
    requiresClass: true
  },
  {
    id: "marksets.open_mark_set",
    group: "marksets",
    label: "Open a Mark Set",
    screenRoute: "marks",
    implemented: true,
    enabledReason: "Opens marks screen.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-marksets-open-markset",
    requiresClass: true
  },
  {
    id: "marksets.edit_heading_categories",
    group: "marksets",
    label: "Edit Heading and Categories",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Opens mark set setup editor.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-marksets-edit-heading-categories",
    requiresClass: true
  },
  {
    id: "marksets.undelete_mark_set",
    group: "marksets",
    label: "Undelete a Mark Set",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Managed from mark set manager.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-marksets-undelete-markset",
    requiresClass: true
  },
  {
    id: "workingon.entry_heading",
    group: "workingon",
    label: "Entry Heading",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Entry heading is managed in mark set setup.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-workingon-entry-heading",
    requiresClass: true
  },
  {
    id: "workingon.edit_marks",
    group: "workingon",
    label: "Edit Marks",
    screenRoute: "marks",
    implemented: true,
    enabledReason: "Opens marks editor.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-workingon-edit-marks",
    requiresClass: true
  },
  {
    id: "workingon.display_print",
    group: "workingon",
    label: "Display/Print",
    screenRoute: "reports",
    implemented: true,
    enabledReason: "Opens reports/export workflows.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-workingon-display-print",
    requiresClass: true
  },
  {
    id: "workingon.clone_entry",
    group: "workingon",
    label: "Clone Entry",
    screenRoute: "marks",
    implemented: true,
    enabledReason: "Clone workflow is in marks action strip.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-workingon-clone-entry",
    requiresClass: true
  },
  {
    id: "reports.mark_set_reports",
    group: "reports",
    label: "Mark Set Reports",
    screenRoute: "reports",
    implemented: true,
    enabledReason: "Opens report exports.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-reports-markset-reports",
    requiresClass: true
  },
  {
    id: "reports.class_analytics",
    group: "reports",
    label: "Class Analytics",
    screenRoute: "class_analytics",
    implemented: true,
    enabledReason: "Opens class analytics.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-reports-class-analytics",
    requiresClass: true
  },
  {
    id: "reports.student_analytics",
    group: "reports",
    label: "Student Analytics",
    screenRoute: "student_analytics",
    implemented: true,
    enabledReason: "Opens student analytics.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-reports-student-analytics",
    requiresClass: true
  },
  {
    id: "reports.combined_analytics",
    group: "reports",
    label: "Combined Analytics",
    screenRoute: "combined_analytics",
    implemented: true,
    enabledReason: "Opens combined analytics.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-reports-combined-analytics",
    requiresClass: true
  },
  {
    id: "comments.remarks_in_marks",
    group: "comments",
    label: "Remarks in Marks",
    screenRoute: "marks",
    implemented: true,
    enabledReason: "In-grid remarks are in marks screen.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-comments-remarks-in-marks",
    requiresClass: true
  },
  {
    id: "comments.comment_sets",
    group: "comments",
    label: "Comment Sets",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Comment sets are in mark set setup.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-comments-comment-sets",
    requiresClass: true
  },
  {
    id: "comments.comment_banks",
    group: "comments",
    label: "Comment Banks",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Comment banks are in mark set setup.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-comments-comment-banks",
    requiresClass: true
  },
  {
    id: "comments.transfer_mode",
    group: "comments",
    label: "Transfer Mode",
    screenRoute: "markset_setup",
    implemented: true,
    enabledReason: "Transfer-mode workflows are in mark set setup.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-comments-transfer-mode",
    requiresClass: true
  },
  {
    id: "setup.analysis_report_options",
    group: "setup",
    label: "Analysis/Report Options",
    screenRoute: "setup_admin",
    implemented: true,
    enabledReason: "Setup/Admin hosts report defaults.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-analysis-report-options"
  },
  {
    id: "setup.calculation_setup",
    group: "setup",
    label: "Calculation Setup",
    screenRoute: "calc_settings",
    implemented: true,
    enabledReason: "Opens calculation settings.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-calculation-setup"
  },
  {
    id: "setup.planner_setup",
    group: "setup",
    label: "Planner Setup",
    screenRoute: "setup_admin",
    implemented: true,
    enabledReason: "Planner defaults live in Setup/Admin.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-planner-setup"
  },
  {
    id: "setup.comments_setup",
    group: "setup",
    label: "Comments Setup",
    screenRoute: "setup_admin",
    implemented: true,
    enabledReason: "Comments defaults live in Setup/Admin.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-comments-setup"
  },
  {
    id: "setup.printer_options",
    group: "setup",
    label: "Printer Options",
    screenRoute: "setup_admin",
    implemented: true,
    enabledReason: "Printer defaults live in Setup/Admin.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-printer-options"
  },
  {
    id: "setup.password_email_setup",
    group: "setup",
    label: "Password + Email Setup",
    screenRoute: "setup_admin",
    implemented: true,
    enabledReason: "Security/email defaults live in Setup/Admin.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-setup-password-email"
  },
  {
    id: "integrations.class_exchange",
    group: "integrations",
    label: "Class Exchange",
    screenRoute: "exchange",
    implemented: true,
    enabledReason: "Opens class exchange tab.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-integrations-class-exchange",
    requiresClass: true
  },
  {
    id: "integrations.sis",
    group: "integrations",
    label: "SIS",
    screenRoute: "exchange",
    implemented: true,
    enabledReason: "Opens SIS integration tab.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-integrations-sis",
    requiresClass: true
  },
  {
    id: "integrations.admin_transfer",
    group: "integrations",
    label: "Admin Transfer",
    screenRoute: "exchange",
    implemented: true,
    enabledReason: "Opens admin transfer tab.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-integrations-admin-transfer",
    requiresClass: true
  },
  {
    id: "planner.units_lessons",
    group: "planner",
    label: "Units + Lessons",
    screenRoute: "planner",
    implemented: true,
    enabledReason: "Opens planner module.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-planner-units-lessons",
    requiresClass: true
  },
  {
    id: "planner.course_description",
    group: "planner",
    label: "Course Description",
    screenRoute: "course_description",
    implemented: true,
    enabledReason: "Opens course description tools.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-planner-course-description",
    requiresClass: true
  },
  {
    id: "planner.planner_reports",
    group: "planner",
    label: "Planner Reports",
    screenRoute: "reports",
    implemented: true,
    enabledReason: "Exports planner/course reports.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-planner-reports",
    requiresClass: true
  },
  {
    id: "help.legacy_actions_map",
    group: "help",
    label: "Legacy Actions Map",
    screenRoute: "legacy_actions_map",
    implemented: true,
    enabledReason: "Shows implemented vs pending legacy actions.",
    pendingReason: "Not implemented yet",
    testId: "legacy-action-help-legacy-actions-map"
  }
];

export const SCREEN_HEADER_ACTION_LABELS: Partial<Record<AppScreen, string>> = {
  marks: "Edit Marks",
  reports: "Mark Set Reports",
  exchange: "Class Exchange",
  planner: "Units + Lessons"
};

export function actionsForGroup(group: LegacyMenuGroupId): LegacyMenuAction[] {
  return LEGACY_MENU_ACTIONS.filter((action) => action.group === group);
}
