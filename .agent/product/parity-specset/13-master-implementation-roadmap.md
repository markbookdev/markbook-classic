# 13 - Master Implementation Roadmap

## Program Strategy
- Core-first parity lane.
- Each wave produces shippable increments and updated traceability.
- No IPC breaking changes unless versioned explicitly.

## Wave 1 - Classroom Workflow Closure (P0)
- Complete remaining core menu/action parity:
  - Working On clone/delete/hide/update-all
  - mark-set transfer/individual-student transfer nuances
  - class update-from-file attach/undelete nuances
- Harden high-frequency marks/remarks flows.
- Deliverables:
  - updated contracts, IPC additions, E2E coverage, gap score updates.

## Wave 2 - Analytics Screens Parity (P0)
- Build class/student/combined interactive analytics tabs.
- Align on-screen analytics with report model outputs.
- Deliverables:
  - analytics APIs, screens, report alignment tests.

## Wave 3 - Comments Transfer and Communication (P0/P1)
- Finish normal vs transfer comment workflows.
- Add compare/import notes/flood-fill parity where missing.
- Deliverables:
  - comments workflow completion + tests.

## Wave 4 - Planner and Course Description (P1)
- Implement chapter 6 planner and publishing surfaces.
- Implement course description/time management generation.
- Deliverables:
  - planner module APIs, schema, screens, export/report support.

## Wave 5 - Setup/Admin Parity (P1)
- Expose setup subdomains affecting analysis, attendance, calc, comments, printer, etc.
- Deliverables:
  - setup APIs and screens, migration tests.

## Wave 6 - Integration Parity Hardening (P2)
- Tier A: SIS/class exchange and admin transfer robustness.
- Tier B: external adapter ecosystems.
- Deliverables:
  - integration matrix and staged adapter implementation.

## Wave 7 - Visual/Discoverability Parity Pass (P2)
- Improve menu IA and command discoverability to legacy mental model.
- Preserve modern architecture and security boundaries.
- Deliverables:
  - UI polish, nav parity checklist, usability regression suite.

## Cross-Cutting Parallel Lane - Parity Evidence
- Maintain regression lock lane always-on.
- Prepare strict legacy-truth lane for immediate activation with fresh outputs.

## Exit Criteria (program-level)
- Every legacy chapter/menu action mapped to implemented/planned/deferred with rationale.
- P0/P1 parity backlog completed with green release gates.
- Strict parity lane activatable without code/test redesign.
