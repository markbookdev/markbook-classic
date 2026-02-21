# 15 - Traceability Matrix

## Purpose
Map legacy references to current implementation, gap status, and planned chunk IDs.

## Matrix
| Legacy reference | Legacy surface | Current implementation anchor | Status | Planned chunk | Acceptance tests |
| --- | --- | --- | --- | --- | --- |
| Ch 1-4 + CLLOAD/CLEDIT | New class and class metadata | `ClassWizardScreen.tsx`, `classes.*` handlers | Partial | EPIC-CORE-02 | `class-wizard.e2e.spec.cjs`, `class-profile.e2e.spec.cjs` |
| Ch 4-1/4-2 | student CRUD/edit | `StudentsScreen.tsx`, `students.*` handlers | Implemented | Maintain | `students-membership.e2e.spec.cjs`, students rust tests |
| Ch 4-4..4-8 | roster import/update from files | `class.importLegacy` + `classes.legacyPreview` + `classes.updateFromLegacy` | Implemented (core) | EPIC-CORE-02 | `classes_update_from_legacy_upsert.rs`, `classes_update_preserve_validity.rs`, `classes_update_collision_policy.rs`, `class-update-from-legacy.e2e.spec.cjs` |
| Ch 4-4..4-8 + MAIN class tools | attached source and one-click reimport | `classes.importLink.get/set` + `classes.updateFromAttachedLegacy` + Dashboard attach/reimport controls | Implemented (core) | EPIC-CORE-03 | `classes_import_link_roundtrip.rs`, `classes_update_from_attached_legacy.rs`, `class-attach-reimport.e2e.spec.cjs` |
| Ch 7 | attendance | `AttendanceScreen.tsx`, `attendance.*` | Implemented | Maintain | `attendance.e2e.spec.cjs` |
| Ch 5-4/5-5 + Ch7-4 | seating | `SeatingPlanScreen.tsx`, `seating.*` | Implemented | Maintain | `seating.e2e.spec.cjs` |
| Ch 5-6/5-7 | loaned items | `LoanedItemsScreen.tsx`, `loaned.*` | Implemented | Maintain | `loaned-items.e2e.spec.cjs` |
| Ch 4-2 + iPad form | device mappings | `DeviceMappingsScreen.tsx`, `devices.*` | Implemented | Maintain | `device-mappings.e2e.spec.cjs` |
| Ch 8-2/8-3/8-5/8-8/8-9 | marks entry/update/weight | `MarksScreen.tsx`, `grid.*`, `assessments.*` | Partial | EPIC-CORE-01 | marks e2e suites |
| Ch 8-4 + MARKSET form | mark set lifecycle | `MarkSetSetupScreen.tsx`, `marksets.*` | Partial | EPIC-CORE-01 | `markset-lifecycle.e2e.spec.cjs` |
| MAIN / MARKSET transfer flows | mark set transfer and merge | `marksets.transfer.preview/apply`, transfer dialog in `MarkSetSetupScreen.tsx` | Implemented (core) | EPIC-CORE-02 | `marksets_transfer_apply.rs`, `markset-transfer.e2e.spec.cjs` |
| MAIN Working On menu | clone/delete/hide/update-all | marks action strip + `entries.*` + `marks.pref.hideDeleted.*` handlers | Implemented | EPIC-CORE-01 | `marks-action-strip.e2e.spec.cjs`, `marks-hide-deleted.e2e.spec.cjs`, `marks-update-all.e2e.spec.cjs`, rust `entries_*` tests |
| Ch 10 + COMMEDIT/ERC/ERCXFER | comments and transfer modes | setup/marks comments workflows + `comments.transfer.preview/apply/floodFill` | Implemented (core transfer) | EPIC-COMMENTS-01 | `comments-transfer-mode.e2e.spec.cjs`, `comments-flood-fill.e2e.spec.cjs`, `comments_transfer_preview.rs`, `comments_transfer_apply_policies.rs`, `comments_flood_fill.rs`, `comments_fit_constraints.rs` |
| Ch 9 class report tabs | interactive class analytics | `ClassAnalyticsScreen.tsx` + `analytics.class.open` + `analytics.class.rows` + `analytics.class.assessmentDrilldown` + `reports.classAssessmentDrilldownModel` | Implemented (interactive read-only) | EPIC-ANALYTICS-01 + EPIC-ANALYTICS-04 | `class-analytics.e2e.spec.cjs`, `class-analytics-interactive.e2e.spec.cjs`, `analytics_class_open.rs`, `analytics_class_rows.rs`, `analytics_assessment_drilldown.rs`, `analytics_drilldown_reports_alignment.rs` |
| Ch 11 student report tabs | interactive student analytics | `StudentAnalyticsScreen.tsx` + `analytics.student.open` + `analytics.student.compare` + `analytics.student.trend` + report handoff | Implemented (interactive read-only) | EPIC-ANALYTICS-02 + EPIC-ANALYTICS-04 | `student-analytics.e2e.spec.cjs`, `student-analytics-compare-trend.e2e.spec.cjs`, `analytics_student_open.rs`, `analytics_student_compare.rs`, `analytics_student_trend.rs` |
| Ch 12 combined report tabs | combined analytics | `CombinedAnalyticsScreen.tsx` + `analytics.combined.open` + `reports.combinedAnalysisModel` | Implemented (read-only) | EPIC-ANALYTICS-02 | `combined-analytics.e2e.spec.cjs`, `combined-analytics-report-alignment.e2e.spec.cjs`, `analytics_combined_open.rs`, `analytics_combined_reports_alignment.rs` |
| Ch 6 planner | unit/lesson planner | no module | Missing | EPIC-PLANNER-01 | new planner tests |
| Ch 6-8/6-11 | course description/time management | no module | Missing | EPIC-PLANNER-02 | new course description tests |
| MAIN Setup menu | setup subdomains | `SetupAdminScreen.tsx` + `CalcSettingsScreen.tsx` + `setup.get/update` (includes additive marks/exchange/analytics defaults) | Partial | EPIC-SETUP-01 + EPIC-SETUP-02 | `setup-admin.e2e.spec.cjs`, `setup_admin_ipc.rs`, `setup_defaults_extended.rs` |
| Setup defaults for transfer/export workflows | integrations/reporting defaults | `setup.get/update` (`setup.integrations`, additive `setup.reports` fields) + `SetupAdminScreen.tsx` | Implemented (core defaults) | EPIC-SETUP-01 | `setup_integrations_defaults.rs`, `setup-admin.e2e.spec.cjs` |
| Ch 3-2/3-3 + BACKUP | backup/restore | `backup.*`, packaged smoke | Partial | EPIC-INTEGRATIONS-01 | backup e2e + packaged smoke |
| Ch 13 exports/integrations | Tier-A SIS + admin transfer | `integrations.sis.*`, `integrations.adminTransfer.*`, upgraded `ExchangeScreen.tsx` tabs | Implemented (Tier-A) | EPIC-INTEGRATIONS-01 | `integrations_sis_preview_apply.rs`, `integrations_sis_exports.rs`, `integrations_admin_transfer_roundtrip.rs`, `integrations_admin_transfer_collision_policy.rs`, `integrations-sis.e2e.spec.cjs`, `integrations-admin-transfer.e2e.spec.cjs` |
| Ch 13 exports/integrations | broader adapter families | deferred beyond Tier-A | Partial | EPIC-INTEGRATIONS-01 | follow-on adapter test plan |
| MAIN Help/menu discoverability | command discoverability parity | AppShell legacy menu blocks | Partial | EPIC-UX-01 | navigation/discoverability e2e |
| Appendix A-7 | calc algorithm parity + evidence lane | `calc.rs`, parity tests, parity manifest/checksum preflight, CI strict-readiness gates | Partial | EPIC-EVIDENCE-01 + EPIC-EVIDENCE-01B | calc parity suites + `parity_fixture_preflight.rs` + parity status checks |

## Coverage Check Rule
Any new feature or menu action added in implementation must include:
- one legacy reference row,
- one current code anchor,
- one chunk ID,
- one acceptance test pointer.
