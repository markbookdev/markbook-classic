# 15 - Traceability Matrix

## Purpose
Map legacy references to current implementation, gap status, and planned chunk IDs.

## Matrix
| Legacy reference | Legacy surface | Current implementation anchor | Status | Planned chunk | Acceptance tests |
| --- | --- | --- | --- | --- | --- |
| Ch 1-4 + CLLOAD/CLEDIT | New class and class metadata | `ClassWizardScreen.tsx`, `classes.*` handlers | Partial | EPIC-CORE-02 | `class-wizard.e2e.spec.cjs`, `class-profile.e2e.spec.cjs` |
| Ch 4-1/4-2 | student CRUD/edit | `StudentsScreen.tsx`, `students.*` handlers | Implemented | Maintain | `students-membership.e2e.spec.cjs`, students rust tests |
| Ch 4-4..4-8 | roster import/update from files | `class.importLegacy`, import handlers | Partial | EPIC-CORE-02 | import integration tests |
| Ch 7 | attendance | `AttendanceScreen.tsx`, `attendance.*` | Implemented | Maintain | `attendance.e2e.spec.cjs` |
| Ch 5-4/5-5 + Ch7-4 | seating | `SeatingPlanScreen.tsx`, `seating.*` | Implemented | Maintain | `seating.e2e.spec.cjs` |
| Ch 5-6/5-7 | loaned items | `LoanedItemsScreen.tsx`, `loaned.*` | Implemented | Maintain | `loaned-items.e2e.spec.cjs` |
| Ch 4-2 + iPad form | device mappings | `DeviceMappingsScreen.tsx`, `devices.*` | Implemented | Maintain | `device-mappings.e2e.spec.cjs` |
| Ch 8-2/8-3/8-5/8-8/8-9 | marks entry/update/weight | `MarksScreen.tsx`, `grid.*`, `assessments.*` | Partial | EPIC-CORE-01 | marks e2e suites |
| Ch 8-4 + MARKSET form | mark set lifecycle | `MarkSetSetupScreen.tsx`, `marksets.*` | Partial | EPIC-CORE-01 | `markset-lifecycle.e2e.spec.cjs` |
| MAIN Working On menu | clone/delete/hide/update-all | marks action strip + `entries.*` + `marks.pref.hideDeleted.*` handlers | Implemented | EPIC-CORE-01 | `marks-action-strip.e2e.spec.cjs`, `marks-hide-deleted.e2e.spec.cjs`, `marks-update-all.e2e.spec.cjs`, rust `entries_*` tests |
| Ch 10 + COMMEDIT/ERC/ERCXFER | comments and transfer modes | comment screens + `comments.*` handlers | Partial | EPIC-COMMENTS-01 | `comments.e2e.spec.cjs`, `marks-remarks.e2e.spec.cjs` |
| Ch 9 class report tabs | interactive class analytics | report models exist, interactive tabs missing | Partial | EPIC-ANALYTICS-01 | new class analytics e2e |
| Ch 11 student report tabs | interactive student analytics | report model exists | Partial | EPIC-ANALYTICS-02 | new student analytics e2e |
| Ch 12 combined report tabs | combined analytics | report model exists | Partial | EPIC-ANALYTICS-02 | new combined analytics e2e |
| Ch 6 planner | unit/lesson planner | no module | Missing | EPIC-PLANNER-01 | new planner tests |
| Ch 6-8/6-11 | course description/time management | no module | Missing | EPIC-PLANNER-02 | new course description tests |
| MAIN Setup menu | setup subdomains | partial via calc settings | Partial | EPIC-SETUP-01 | setup-specific tests |
| Ch 3-2/3-3 + BACKUP | backup/restore | `backup.*`, packaged smoke | Partial | EPIC-INTEGRATIONS-01 | backup e2e + packaged smoke |
| Ch 13 exports/integrations | external adapter breadth | exchange/SIS partial | Partial | EPIC-INTEGRATIONS-01 | exchange tests + adapter tests |
| MAIN Help/menu discoverability | command discoverability parity | AppShell legacy menu blocks | Partial | EPIC-UX-01 | navigation/discoverability e2e |
| Appendix A-7 | calc algorithm parity | `calc.rs`, parity tests | Partial | EPIC-EVIDENCE-01 + analytics chunks | calc parity suites |

## Coverage Check Rule
Any new feature or menu action added in implementation must include:
- one legacy reference row,
- one current code anchor,
- one chunk ID,
- one acceptance test pointer.
