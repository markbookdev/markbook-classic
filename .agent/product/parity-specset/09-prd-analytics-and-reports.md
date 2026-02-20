# 09 - PRD: Analytics and Reports

## Product Goal
Provide interactive class/student/combined analytics parity with legacy while preserving the current PDF export architecture.

## User Problems
- Teachers need to inspect trends/distributions before publishing reports.
- Current exports exist, but interactive parity with legacy report screens is incomplete.

## In Scope
- Interactive analytics screens for class, student, and combined perspectives.
- Report model fidelity and filter alignment with marks screen.
- Extended report templates that remain print-grade across multipage/wide data.

## Out of Scope
- Replacing Chromium `printToPDF` pipeline.
- Non-core external report channels.

## Functional Requirements

## FR-1 Class analytics parity
- Summary, entries, block/unit, category, distribution, modal, seating, compare views.
- Ability to adjust overall marks where legacy supports that workflow.

## FR-2 Student analytics parity
- Entries, unit, category, trend, modal-level, compare-calc-method views.

## FR-3 Combined analytics parity
- Combined summary/distribution/modal/seating/set-weighting/class-report/student-tab workflows.

## FR-4 Filter and scope consistency
- Marks and report models must share the same filter semantics:
  - term, category, types mask, student scope.

## FR-5 Export fidelity
- PDF exports must reflect on-screen computed values and applied filters.
- Repeated headers, stable pagination, wide-table safety.

## API Requirements
- Use existing `calc.*` and `reports.*Model` families as base.
- Additive interactive analytics endpoints may be introduced under `reports.class.*`, `reports.student.*`, `reports.combined.*`.
- No required-field breaking changes.

## Data/Calc Requirements
- All analytics derive from sidecar calc engine with parity locks.
- No renderer-side computed aggregates for authoritative values.

## Acceptance Criteria
- Interactive tabs produce values that match exported model values for same inputs.
- Existing report export E2E plus new analytics-screen E2E all pass.
- Regression and strict parity lanes remain runnable.
