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
  - Rust action semantics + Playwright marks-action-strip expansion.
- Acceptance:
  - all Working On actions functional and persistent.

## EPIC-CORE-02: Mark set transfer and class update parity details
- Objective:
  - close remaining transfer/update-from-file nuances.
- File targets:
  - `rust/markbookd/src/legacy.rs`
  - `rust/markbookd/src/ipc/handlers/import_legacy.rs`
  - `apps/desktop/src/renderer/ui/screens/StudentsScreen.tsx`
- Tests:
  - import/update integration tests + e2e import workflows.

## EPIC-ANALYTICS-01: Class analytics interactive tabs
- Objective:
  - deliver chapter-9 equivalent screen behaviors.
- File targets:
  - new screens/components under `apps/desktop/src/renderer/ui/screens/`
  - `rust/markbookd/src/ipc/handlers/reports.rs`
- Tests:
  - analytics tab e2e + model alignment tests.

## EPIC-ANALYTICS-02: Student + combined analytics parity
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
