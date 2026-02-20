# 08 - PRD: Classroom Core

## Product Goal
Deliver a classroom-ready parity core where daily teacher workflows in class management and grading are equivalent in function to legacy MarkBook.

## Target Users
- Classroom teachers using MarkBook daily for roster, attendance, mark entry, comments, and report preparation.
- Department leads requiring reliable grade computation and auditability.

## In Scope
- Class lifecycle (create/edit/open/delete; import legacy class folders).
- Student lifecycle (CRUD/reorder/active/membership).
- Mark set lifecycle and setup.
- Fast marks entry and bulk operations.
- Attendance, seating, notes, loaned items, device mappings.
- Comments/remarks in grading context.

## Out of Scope (for this PRD)
- Planner and course description modules.
- Peripheral chapter-13 adapter ecosystems beyond current exchange/SIS lane.
- Pixel-perfect visual clone.

## Functional Requirements

## FR-1 Class and student lifecycle parity
- Must support all core class and student operations from legacy menus.
- Must keep stable `sort_order` mapping for every student-facing grid/report.

## FR-2 Mark set and assessment lifecycle parity
- Must allow create/open/clone/default/delete/undelete mark sets.
- Must support single/bulk assessment creation/update flows.

## FR-3 Classroom-speed marks workflow
- Keyboard-first editing with deterministic commit navigation.
- Bulk update diagnostics must be explicit and actionable.
- Zero/no_mark semantics must remain locked.

## FR-4 Membership-aware calculations
- Membership toggles must immediately affect calc/report outputs.
- Bulk membership operations must be one-call and low latency.

## FR-5 Remarks without context switching
- Selected student remark save/update in marks screen.
- Bank-assisted insertion with append/replace behavior.

## FR-6 Attendance and seating parity baseline
- Monthly attendance + day-type and batch stamps.
- Seating assignment/blocked-seat persistence and report compatibility.

## FR-7 Data safety and compatibility
- Legacy import must preserve essential semantics from CL/Yxx/companions.
- Migrations must preserve old workspace readability.

## UX Requirements
- Existing shell remains functional; legacy discoverability layer improves command findability.
- Disabled parity-pending actions show explicit reason text.
- No blocking modal loops for high-frequency grading actions.

## API Requirements (additive only)
- Extend under existing namespaces where possible (`students.*`, `assessments.*`, `grid.*`, `comments.*`).
- Return structured errors with stable `code/message/details`.
- Keep schema validation strict in renderer.

## Non-Functional Requirements
- Preserve renderer security boundary.
- Keep large mark sets performant via windowed fetch.
- Ensure packaged app still passes launch smoke gates.

## Success Metrics
- Teacher can run full daily cycle without leaving core screens.
- No critical regressions in marks edit, attendance, seating, and report export E2E suites.
- Parity regression lane remains green after each chunk.

## Acceptance Criteria
- Core classroom workflows pass Playwright and parity-regression gates.
- No IPC breaking changes.
- No migration failures on existing snapshots.
