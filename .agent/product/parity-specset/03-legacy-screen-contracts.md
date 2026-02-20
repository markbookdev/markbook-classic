# 03 - Legacy Screen Contracts

## Purpose
Decision-complete functional contracts for parity-critical screens and workflows.

## Contract Template (applies to every entry)
- Legacy references: chapter/section/page + `.FRM` + `.BAS` procedures.
- User intent and success outcome.
- Controls/actions/menu entry points.
- Data/IPC contracts.
- Validation and failure behavior.
- Calc/report dependencies.
- Acceptance tests.
- Status + priority.
- Migration/compat constraints.

## Contract 1: Main Shell and Menu Parity
- Legacy refs:
  - `MarkBook_Reference.pdf` chapter 3 (3-1).
  - `MAIN.FRM` top-level menus.
  - `MB_V12.BAS` form launch routing procedures.
- User intent:
  - Access all daily and periodic workflows from predictable top-level menus.
- Controls/actions:
  - File/Class/Mark Sets/Working On/Reports/Comments/Tools/Setup/Help menu groups.
- Data/IPC contracts:
  - No new required payload shape; menu actions dispatch to existing or new additive IPC methods.
- Validation/failure:
  - Unimplemented action must show explicit disabled state + reason.
- Calc/report dependencies:
  - None directly, but navigation to analytics/reports requires parity-locked calc.
- Acceptance tests:
  - Playwright menu discoverability test with implemented vs disabled assertions.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - Keep current AppShell navigation operational; add menu parity layer without removing existing routes.

## Contract 2: Class Wizard + Class Profile
- Legacy refs:
  - Chapter 1-4, chapter 2-2.
  - `CLLOAD.FRM`, `CLEDIT.FRM`.
- User intent:
  - Create/edit class metadata and defaults with guided flow.
- Controls/actions:
  - Wizard steps (class identity, defaults, review), profile edit mode.
- Data/IPC contracts:
  - `classes.wizardDefaults`
  - `classes.createFromWizard`
  - `classes.meta.get`
  - `classes.meta.update`
- Validation/failure:
  - Required identity fields; duplicate class-code handling.
- Calc/report dependencies:
  - Default calc settings pre-seed new mark sets.
- Acceptance tests:
  - Rust meta persistence tests + Playwright class-wizard/profile specs.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - Preserve `classes.create` for backward compatibility.

## Contract 3: Student Lifecycle and SIS Update Flow
- Legacy refs:
  - Chapter 4 (4-1, 4-2, 4-4..4-9).
  - `IMPORT.FRM`, `StudentTrans.frm`.
- User intent:
  - Maintain roster accurately, import updates safely, preserve ordering and membership rules.
- Controls/actions:
  - Add/edit/delete/reorder, active/inactive, mark-set membership matrix.
- Data/IPC contracts:
  - `students.list/create/update/reorder/delete`
  - `students.membership.get/set/bulkSet`
  - `class.importLegacy` for import/update lanes.
- Validation/failure:
  - Permutation validation for reorder, idempotent updates, deterministic errors.
- Calc/report dependencies:
  - `valid_kid` semantics depend on active + membership mask.
- Acceptance tests:
  - membership and reorder E2E + calc impact integration tests.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - membership edits convert `TBA` to explicit mask safely.

## Contract 4: Mark Set Lifecycle
- Legacy refs:
  - Chapter 8 (8-4), chapter 2-2.
  - `MARKSET.FRM`, `MAIN.FRM` Mark Sets menu.
- User intent:
  - Create/open/clone/default/delete/undelete mark sets with reliable ordering.
- Controls/actions:
  - New mark set wizard, clone options, include-deleted manager, default marker.
- Data/IPC contracts:
  - `marksets.list/create/delete/undelete/setDefault/clone`
  - `markset.settings.get/update`
- Validation/failure:
  - code uniqueness per class; soft-delete constraints when selected/open.
- Calc/report dependencies:
  - mark set sort order and settings affect calc and report scope.
- Acceptance tests:
  - Rust lifecycle tests + Playwright lifecycle spec.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - existing IDs remain stable; soft-delete only.

## Contract 5: Marks Entry Workflow (single + multiple)
- Legacy refs:
  - Chapter 8 (8-2, 8-3, 8-5, 8-8, 8-9).
  - `MRKENTRY.FRM`, `MKUPDATE.FRM`, `WT_BONUS.FRM`, `HEADING.FRM`.
- User intent:
  - Fast keyboard grading with batch updates and deterministic semantics.
- Controls/actions:
  - New Entry, Multiple New, Entry Update, Weight, Multiple Update, edit grid.
- Data/IPC contracts:
  - `grid.get/updateCell/setState/bulkUpdate`
  - `assessments.create/bulkCreate/update/bulkUpdate/reorder/delete`
- Validation/failure:
  - direct blank/0 -> no_mark; explicit zero action -> zero state.
  - bulk diagnostics: updated/rejected/errors.
- Calc/report dependencies:
  - all updates must reflect in `calc.assessmentStats` and `calc.markSetSummary`.
- Acceptance tests:
  - marks edit/bulk E2E, range validation, bulk limits.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - no breaking semantics for existing score statuses.

## Contract 6: Attendance
- Legacy refs:
  - Chapter 7.
  - `ATTEND.FRM`.
- User intent:
  - Monthly attendance with day-type codes and quick stamping.
- Controls/actions:
  - month open, type-of-day row, per-student day edit, bulk stamp.
- Data/IPC contracts:
  - `attendance.monthOpen/setTypeOfDay/setStudentDay/bulkStampDay`
- Validation/failure:
  - day bounds, code validation, month-key format.
- Calc/report dependencies:
  - attendance summaries used by class report and attendance reports.
- Acceptance tests:
  - attendance E2E persistence + report export.
- Status/Priority:
  - Implemented, P0.
- Migration/compat:
  - support `.ATN` and fallback strategy for `.ATT` where applicable.

## Contract 7: Seating Plan
- Legacy refs:
  - Chapter 5-4/5-5 and 7-4.
  - `SEATPLAN.FRM`.
- User intent:
  - Build/manage seating map and use it for attendance/report analysis.
- Controls/actions:
  - assign/unassign, block seats, auto-place modes.
- Data/IPC contracts:
  - `seating.get/save`
- Validation/failure:
  - matrix dimensions and assignment uniqueness.
- Calc/report dependencies:
  - report seating views consume seating assignments.
- Acceptance tests:
  - seating E2E persistence.
- Status/Priority:
  - Implemented, P0.
- Migration/compat:
  - `.SPL` import preserved.

## Contract 8: Loaned Items and Device Mappings
- Legacy refs:
  - Chapter 5-6/5-7.
  - `CLPRINT.FRM`, `iPad.frm`.
- User intent:
  - Track issued materials and per-student device identifiers.
- Controls/actions:
  - create/update loaned items, edit/clear device codes.
- Data/IPC contracts:
  - `loaned.list/get/update`
  - `devices.list/get/update`
- Validation/failure:
  - immutable student linkage constraints.
- Calc/report dependencies:
  - indirectly surfaced in list/print forms.
- Acceptance tests:
  - loaned/device E2E specs.
- Status/Priority:
  - Implemented, P1.
- Migration/compat:
  - `.TBK` / `.ICC` import compatibility maintained.

## Contract 9: Notes and Comments Workflows
- Legacy refs:
  - Chapter 3-4 and chapter 10.
  - `KIDNOTES.FRM`, `ERC.FRM`, `ERCXFER.FRM`, `COMMEDIT.FRM`.
- User intent:
  - create and apply high-quality comments rapidly in normal and transfer contexts.
- Controls/actions:
  - remarks in marks context, comment sets, bank filtering, append/replace, flood-fill, compare/import.
- Data/IPC contracts:
  - `comments.sets.*`, `comments.banks.*`, `comments.remarks.upsertOne`, `notes.get/update`.
- Validation/failure:
  - fit constraints, per-set defaults, safe delete behavior.
- Calc/report dependencies:
  - student/combined report models consume remark content.
- Acceptance tests:
  - comments E2E + marks-remarks E2E.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - maintain `.IDX/.R*` and `ALL!*.IDX` merge semantics.

## Contract 10: Class/Student/Combined Analytics Screens
- Legacy refs:
  - Chapters 9, 11, 12.
  - `EVC.FRM`, `EVI.FRM`, `EVA.FRM`.
- User intent:
  - inspect results through summary, distribution, compare, category, trend, seating tabs.
- Controls/actions:
  - interactive analytics tabs and filters matching legacy intent.
- Data/IPC contracts:
  - existing `calc.*` and `reports.*Model` plus new interactive analytics APIs.
- Validation/failure:
  - filter behavior must be deterministic and mirrored in export headers.
- Calc/report dependencies:
  - parity-locked calc engine mandatory.
- Acceptance tests:
  - alignment tests between on-screen model and PDF model.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - additive endpoints only.

## Contract 11: Planner and Course Description
- Legacy refs:
  - Chapter 6 and tools menu entries.
  - `TTDISPLY.FRM` + planner/curriculum docs.
- User intent:
  - create, manage, and publish unit/lesson/course plans.
- Controls/actions:
  - planner list, unit summary, lesson outline/detail/follow-up, publishing states.
- Data/IPC contracts:
  - planned `planner.units.*`, `planner.lessons.*`, `planner.publish.*`, `courseDescription.*`.
- Validation/failure:
  - required structure for publishable plans.
- Calc/report dependencies:
  - none direct to marks calc.
- Acceptance tests:
  - create/edit/publish E2E + model persistence tests.
- Status/Priority:
  - Missing, P1.
- Migration/compat:
  - separate tables/modules to avoid marks schema risk.

## Contract 12: Backup, Restore, Exchange
- Legacy refs:
  - Chapter 3-2/3-3 and chapter 13.
  - `BACKUP.FRM`, `Export.frm`.
- User intent:
  - recover classes safely and move data to/from external systems.
- Controls/actions:
  - bundle export/import, CSV exchange import/export.
- Data/IPC contracts:
  - `backup.exportWorkspaceBundle/importWorkspaceBundle`
  - `exchange.exportClassCsv/importClassCsv`
- Validation/failure:
  - manifest verification, legacy compatibility import path.
- Calc/report dependencies:
  - integrity gate for all domains.
- Acceptance tests:
  - packaged smoke + backup/restore E2E.
- Status/Priority:
  - Partial, P0.
- Migration/compat:
  - support legacy raw sqlite restore path where already implemented.
