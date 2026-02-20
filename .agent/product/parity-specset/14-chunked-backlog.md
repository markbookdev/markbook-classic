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
  - IN PROGRESS (2026-02-20): backend `analytics.class.open`/`analytics.filters.options` + read-only class analytics screen + report handoff wiring.
- Objective:
  - deliver chapter-9 equivalent screen behaviors.
- File targets:
  - new screens/components under `apps/desktop/src/renderer/ui/screens/`
  - `rust/markbookd/src/ipc/handlers/reports.rs`
- Tests:
  - analytics tab e2e + model alignment tests.

## EPIC-ANALYTICS-02: Student + combined analytics parity
- Status:
  - IN PROGRESS (2026-02-20): backend `analytics.student.open` + read-only student analytics screen shipped; combined analytics remains next slice.
- Objective:
  - deliver chapter 11/12 interactive parity.
- Tests:
  - parity of values against report models and calc endpoints.

## EPIC-COMMENTS-01: Transfer-mode + compare/import/flood-fill
- Objective:
  - close chapter-10 workflow gaps.
- File targets:
  - `apps/desktop/src/renderer/ui/screens/MarkSetSetupScreen.tsx`
  - `apps/desktop/src/renderer/ui/screens/MarksScreen.tsx`
  - `rust/markbookd/src/ipc/handlers/comments.rs`
- Tests:
  - comments transfer-mode e2e and remark propagation checks.

## EPIC-PLANNER-01: Unit/lesson planner MVP
- Objective:
  - core planner data model + CRUD screens.
- API changes:
  - `planner.units.*`, `planner.lessons.*`
- Tests:
  - persistence and workflow e2e.

## EPIC-PLANNER-02: Publish and course description
- Objective:
  - publishing states and course-description generator.
- API changes:
  - `planner.publish.*`, `courseDescription.*`

## EPIC-SETUP-01: Setup subdomain surfaces
- Objective:
  - expose calc/comments/attendance/printer/email/password options.
- API changes:
  - `setup.*` namespaces.

## EPIC-INTEGRATIONS-01: SIS/admin transfer hardening
- Objective:
  - chapter-13 Tier A parity closure.
- API changes:
  - `integrations.sis.*`, `integrations.adminTransfer.*`.

## EPIC-UX-01: Discoverability parity pass
- Objective:
  - map legacy command expectations into modern shell UX.
- Tests:
  - menu IA and navigation e2e coverage.

## EPIC-EVIDENCE-01: Strict lane activation pack
- Objective:
  - ingest fresh legacy exports and flip strict gates on.
- file targets:
  - `fixtures/legacy/Sample25/expected/fresh-markfiles/...`
  - strict parity tests.
