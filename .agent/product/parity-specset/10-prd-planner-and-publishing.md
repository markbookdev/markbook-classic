# 10 - PRD: Planner and Publishing

## Product Goal
Implement legacy planner and course-description capabilities (chapter 6 and tools menu) as first-class modules.

## In Scope
- Unit planner and lesson planner workflows:
  - Unit summary
  - Lesson outline/detail/follow-up
- Standards/content banks used by planner.
- Publishing and management of lesson/unit/course-description outputs.
- Course description + time management generator.

## Out of Scope
- Broader CMS integrations not present in legacy baseline.

## Functional Requirements
- Create/edit/archive unit plans.
- Create/edit lesson plans linked to units.
- Attach standards and reusable bank items.
- Publish outputs with status metadata and versioning.
- Generate course description documents from planner data and schedule assumptions.

## Data Model Requirements
- Separate planner schema namespace/tables to isolate risk from marks core.
- Preserve workspace-local offline model.
- Support import/export of planner artifacts in future bundle versions.

## API Requirements (planned)
- `planner.units.*`
- `planner.lessons.*`
- `planner.publish.*`
- `courseDescription.*`

## UX Requirements
- Planner screens accessible from Tools menu parity group.
- Draft/published state visibility and edit-lock semantics.

## Testing
- Rust model tests for planner persistence and relationships.
- Playwright workflow tests for create/edit/publish flows.
- PDF/export snapshot checks for generated documents.

## Acceptance Criteria
- Teacher can complete unit/lesson/course-description cycle in-app.
- Planner outputs persist and can be re-opened/updated safely.
