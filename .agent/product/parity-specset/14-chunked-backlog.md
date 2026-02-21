# 14 - Chunked Backlog

## Purpose
Execution-ready chunk list for iterative delivery.

## Chunk Conventions
- `EPIC-<domain>-<nn>`
- each chunk includes:
  - objective
  - file targets
  - API changes
  - tests
  - acceptance criteria

## EPIC-CORE-01: Working On action completion
- Status:
  - DONE (2026-02-20): clone/delete/hide/update-all semantics shipped with backend + marks UI + tests.
- Objective:
  - Complete clone/delete/hide/update-all parity semantics.
- File targets:
  - `apps/desktop/src/renderer/ui/screens/MarksScreen.tsx`
  - `rust/markbookd/src/ipc/handlers/grid.rs`
  - `rust/markbookd/src/ipc/handlers/markset_setup.rs`
  - `packages/schema/src/index.ts`
- API changes:
  - additive entries/assessment action endpoints if needed.
- Tests:
  - Rust: `entries_delete_semantics.rs`, `entries_clone_roundtrip.rs`, `assessments_hide_deleted_like.rs`
  - Playwright: `marks-action-strip.e2e.spec.cjs`, `marks-hide-deleted.e2e.spec.cjs`, `marks-update-all.e2e.spec.cjs`
- Acceptance:
  - all Working On actions functional and persistent.

## EPIC-CORE-02: Mark set transfer and class update parity details
- Status:
  - DONE (2026-02-20): update-from-legacy preview/apply + markset transfer preview/apply shipped with UI + tests.
- Objective:
  - close remaining transfer/update-from-file nuances.
- File targets:
  - `rust/markbookd/src/legacy.rs`
  - `rust/markbookd/src/ipc/handlers/import_legacy.rs`
  - `rust/markbookd/src/ipc/handlers/markset_setup.rs`
  - `apps/desktop/src/renderer/ui/screens/DashboardScreen.tsx`
  - `apps/desktop/src/renderer/ui/screens/StudentsScreen.tsx`
  - `apps/desktop/src/renderer/ui/screens/MarkSetSetupScreen.tsx`
  - `apps/desktop/src/renderer/ui/app/AppShell.tsx`
- API changes:
  - `classes.legacyPreview`
  - `classes.updateFromLegacy`
  - `marksets.transfer.preview`
  - `marksets.transfer.apply`
- Locked defaults:
  - update mode: `upsert_preserve`
  - transfer collision policy: `merge_existing`
  - preserve matched local `active` + `mark_set_mask`: `true`
- Tests:
  - Rust: `classes_update_from_legacy_upsert.rs`, `classes_update_preserve_validity.rs`, `classes_update_collision_policy.rs`, `marksets_transfer_apply.rs`, `db_class_meta_import_link_migration.rs`
  - Playwright: `class-update-from-legacy.e2e.spec.cjs`, `markset-transfer.e2e.spec.cjs`, extended `students-membership.e2e.spec.cjs`
- Acceptance:
  - Existing classes can be previewed and updated in-place from legacy folders with deterministic merge diagnostics.
  - Mark set transfer supports preview and apply with merge/append/stop collision policies and sort-order row alignment warnings.

## EPIC-ANALYTICS-01: Class analytics interactive tabs
- Status:
  - DONE (2026-02-20): `analytics.class.open`/`analytics.filters.options` shipped with read-only class analytics screen + report handoff.
  - DONE (2026-02-20): interactive closure shipped via `analytics.class.rows` and `analytics.class.assessmentDrilldown`, including cohort drilldown and report-model alignment.
- Objective:
  - deliver chapter-9 equivalent screen behaviors.
- File targets:
  - new screens/components under `apps/desktop/src/renderer/ui/screens/`
  - `rust/markbookd/src/ipc/handlers/reports.rs`
- Tests:
  - analytics tab e2e + model alignment tests.

## EPIC-ANALYTICS-02: Student + combined analytics parity
- Status:
  - DONE (2026-02-20): `analytics.student.open`, `analytics.combined.options`, and `analytics.combined.open` shipped with read-only student/combined analytics screens and report alignment.
  - DONE (2026-02-20): student interactive closure shipped via `analytics.student.compare` and `analytics.student.trend`.
- Objective:
  - deliver chapter 11/12 interactive parity.
- Tests:
  - parity of values against report models and calc endpoints.

## EPIC-COMMENTS-01: Transfer-mode + compare/import/flood-fill
- Status:
  - DONE (2026-02-20): `comments.transfer.preview/apply/floodFill` shipped with setup transfer modal, flood-fill UX, fit/max-length enforcement, and diagnostics.
- Objective:
  - close chapter-10 workflow gaps.
- File targets:
  - `apps/desktop/src/renderer/ui/screens/MarkSetSetupScreen.tsx`
  - `apps/desktop/src/renderer/ui/screens/MarksScreen.tsx`
  - `rust/markbookd/src/ipc/handlers/comments.rs`
- Tests:
  - Rust: `comments_transfer_preview.rs`, `comments_transfer_apply_policies.rs`, `comments_flood_fill.rs`, `comments_fit_constraints.rs`
  - Playwright: `comments-transfer-mode.e2e.spec.cjs`, `comments-flood-fill.e2e.spec.cjs`

## EPIC-PLANNER-01: Unit/lesson planner MVP
- Status:
  - DONE (2026-02-21): planner units/lessons class-scoped model + IPC + UI + e2e/regression gates shipped.
- Objective:
  - core planner data model + CRUD screens.
- API changes:
  - `planner.units.*`, `planner.lessons.*`
- Tests:
  - persistence and workflow e2e.

## EPIC-PLANNER-02: Publish and course description
- Status:
  - DONE (2026-02-21): planner publish flows and course-description/time-management model generation + report exports shipped.
- Objective:
  - publishing states and course-description generator.
- API changes:
  - `planner.publish.*`, `courseDescription.*`

## EPIC-SETUP-01: Setup subdomain surfaces
- Status:
  - DONE (2026-02-21): setup surfaces for integrations/report defaults are shipped and wired in Setup/Admin.
- Objective:
  - expose calc/comments/attendance/printer/email/password options.
- API changes:
  - `setup.*` namespaces.

## EPIC-INTEGRATIONS-01: SIS/admin transfer hardening
- Status:
  - DONE (2026-02-21): Tier-A integrations contracts are shipped in sidecar + renderer (`integrations.sis.*`, `integrations.adminTransfer.*`) with preview-first workflows and setup-driven defaults.
- Objective:
  - chapter-13 Tier A parity closure.
- API changes:
  - `integrations.sis.*`, `integrations.adminTransfer.*`.

## EPIC-UX-01: Discoverability parity pass
- Status:
  - DONE (core) (2026-02-21): legacy menu groups are present in AppShell (File/Class/Mark Sets/Working On/Reports/Comments/Setup/Integrations/Planner) with implemented actions wired and pending actions explicitly disabled.
- Objective:
  - map legacy command expectations into modern shell UX.
- Tests:
  - menu IA and navigation e2e coverage.

## EPIC-CORE-03: Attached class reimport parity
- Status:
  - DONE (2026-02-21): class attach/reimport link semantics shipped with dashboard controls and import-link metadata visibility.
- Objective:
  - close attach-source and re-import workflows on top of core update-from-legacy engine.
- API changes:
  - `classes.importLink.get`
  - `classes.importLink.set`
  - `classes.updateFromAttachedLegacy`
- Tests:
  - Rust: `classes_import_link_roundtrip.rs`, `classes_update_from_attached_legacy.rs`
  - Playwright: `class-attach-reimport.e2e.spec.cjs`

## EPIC-SETUP-02: Setup defaults breadth expansion
- Status:
  - DONE (2026-02-21): additive setup defaults for marks/exchange/analytics are shipped and wired into marks/exchange/analytics screens.
- Objective:
  - expose parity-critical operational defaults that affect day-to-day workflows.
- API changes:
  - `setup.marks.defaultHideDeletedEntries`
  - `setup.marks.defaultAutoPreviewBeforeBulkApply`
  - `setup.exchange.defaultExportStudentScope`
  - `setup.exchange.includeStateColumnsByDefault`
  - `setup.analytics.defaultPageSize`
  - `setup.analytics.defaultCohortMode`
- Tests:
  - Rust: `setup_defaults_extended.rs`
  - Playwright: extended `setup-admin.e2e.spec.cjs`

## EPIC-EVIDENCE-01: Strict lane activation pack
- Objective:
  - ingest fresh legacy exports and flip strict gates on.
- file targets:
  - `fixtures/legacy/Sample25/expected/fresh-markfiles/...`
  - strict parity tests.

## EPIC-ANALYTICS-04: Class/student interactive parity closure
- Status:
  - DONE (2026-02-20): additive interactive analytics APIs + UI shipped, with drilldown report parity.
- Objective:
  - close remaining class/student interactive analytics parity while keeping analytics read-only.
- API changes:
  - `analytics.class.rows`
  - `analytics.class.assessmentDrilldown`
  - `analytics.student.compare`
  - `analytics.student.trend`
  - `reports.classAssessmentDrilldownModel`
- Tests:
  - Rust: `analytics_class_rows.rs`, `analytics_assessment_drilldown.rs`, `analytics_student_compare.rs`, `analytics_student_trend.rs`, `analytics_drilldown_reports_alignment.rs`
  - Playwright: `class-analytics-interactive.e2e.spec.cjs`, `student-analytics-compare-trend.e2e.spec.cjs`, `analytics-drilldown-report-alignment.e2e.spec.cjs`

## EPIC-EVIDENCE-01B: Strict-lane readiness hardening
- Status:
  - DONE (readiness) (2026-02-21): manifest checksums + parity status JSON + CI readiness plumbing shipped, plus manifest schema-version checks and machine-readable artifact path summaries.
- Objective:
  - make strict legacy-truth activation deterministic as soon as fresh artifacts arrive.
- File targets:
  - `apps/desktop/scripts/parity-status.cjs`
  - `fixtures/legacy/Sample25/expected/parity-manifest.json`
  - `rust/markbookd/tests/parity_fixture_preflight.rs`
  - `.github/workflows/quality-gates.yml`

## EPIC-SETUP-03: Setup/admin breadth closure
- Status:
  - DONE (core breadth) (2026-02-21): additive attendance/comments/reports/security/printer setup defaults are shipped and wired to consuming screens.
- Objective:
  - close remaining setup domains that affect day-to-day defaults and report output.
- API changes:
  - additive `setup.attendance.*`, `setup.comments.*`, `setup.reports.*`, `setup.security.*`, `setup.printer.*` fields under existing `setup.get/setup.update`.
- Tests:
  - Rust: `setup_sections_depth.rs`
  - Playwright: `setup-admin-depth.e2e.spec.cjs`

## EPIC-PLANNER-03: Planner legacy-depth closure
- Status:
  - DONE (core depth) (2026-02-21): planner clone/copy-forward/bulk-assign and course description generation options are shipped.
- Objective:
  - move planner from MVP into classroom-speed legacy-usable depth.
- API changes:
  - `planner.units.clone`
  - `planner.lessons.copyForward`
  - `planner.lessons.bulkAssignUnit`
  - additive `courseDescription.generateModel` options (`includePolicy`, `includeStrands`, `includeAssessmentPlan`, `includeResources`)
- Tests:
  - Rust: `planner_units_clone.rs`, `planner_lessons_copy_forward.rs`, `course_description_options.rs`
  - Playwright: `planner-clone-copyforward.e2e.spec.cjs`, `course-description-options.e2e.spec.cjs`

## EPIC-UX-02: Discoverability final pass
- Status:
  - IN PROGRESS (2026-02-21): deterministic action-state labeling is in place; final visual/menu parity polish remains.
- Objective:
  - complete discoverability parity without reintroducing legacy security/UI constraints.
- Tests:
  - Playwright: `menu-discoverability.e2e.spec.cjs`
