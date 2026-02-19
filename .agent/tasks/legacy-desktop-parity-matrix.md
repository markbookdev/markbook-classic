# Legacy -> Desktop Parity Matrix

Last updated: 2026-02-19

Legend:
- `Implemented`: Available end-to-end in desktop rewrite.
- `Partial`: Core exists, but parity details or discoverability still incomplete.
- `Missing`: Not yet implemented in desktop rewrite.
- `Deferred`: Intentionally deferred in current classroom-critical track.

## Class and Startup Flows

| Legacy Form/Menu | Desktop Equivalent | Status | Notes |
| --- | --- | --- | --- |
| `CLLOAD.FRM` welcome/open/new-class wizard | `ClassWizardScreen.tsx` + dashboard quick create | Partial | Wizard added; more legacy setup tabs can be layered in. |
| `File -> Make a New Class` | AppShell legacy menu + wizard route | Partial | Wizard flow available; full legacy visuals deferred. |
| `Open Existing/Sample Class` | Workspace + import flow | Partial | Legacy sample/open browser parity still simplified. |
| Class metadata/profile setup | `classes.meta.*` IPC | Partial | API available; rich UI editing not yet surfaced. |

## Mark Set Lifecycle

| Legacy Form/Menu | Desktop Equivalent | Status | Notes |
| --- | --- | --- | --- |
| `Make a New Mark Set` | `marksets.create` + manager panel in `MarkSetSetupScreen` | Implemented | Includes starter category option. |
| `Delete a Mark Set` | `marksets.delete` soft delete | Implemented | Soft delete with undelete support. |
| `Undelete a Mark Set` | `marksets.undelete` | Implemented | Visible in Mark Set Manager. |
| `Make current mark set default` | `marksets.setDefault` | Implemented | Tracks `is_default`. |
| `Clone Mark Set` | `marksets.clone` | Partial | Clones setup/categories/assessments; score clone optional. |
| `More Mark Sets...` and deleted-set list | `marksets.list(includeDeleted)` | Partial | API/UI exists; legacy list UX still simplified. |

## Entry/Marks Workflow

| Legacy Form/Menu | Desktop Equivalent | Status | Notes |
| --- | --- | --- | --- |
| `New Entry` | Marks action strip -> `assessments.create` | Implemented | Quick prompt flow. |
| `Multiple New Entries` | Marks action strip -> `assessments.bulkCreate` | Implemented | Bulk adds headings quickly. |
| `Update Marks` | Grid edit + bulk state tools | Implemented | Persisted SQLite edits with parity mark states. |
| `Update Multiple Entries` | Marks action strip -> `assessments.bulkUpdate` | Implemented | Bulk assessment metadata updates. |
| `Entry Heading` / `Weight` buttons | Action strip routes to setup | Partial | Navigation parity done; richer modal parity pending. |
| `Open a Mark Set` button in main form | Sidebar markset selector + action strip hint | Partial | Functional, not legacy-identical UI. |

## Classroom-Critical Screens

| Legacy Area | Desktop Equivalent | Status | Notes |
| --- | --- | --- | --- |
| Students | `StudentsScreen.tsx` | Implemented | CRUD, reorder, active toggle, membership masks. |
| Mark Set Setup + Categories + Assessments | `MarkSetSetupScreen.tsx` | Implemented | Includes comments/banks tab and lifecycle manager. |
| Attendance | `AttendanceScreen.tsx` | Implemented | Month matrix + type-of-day + bulk stamp. |
| Seating | `SeatingPlanScreen.tsx` | Implemented | Assignments, blocked seats, auto-place modes. |
| Notes | `NotesScreen.tsx` | Implemented | Per-student note persistence. |
| Learning Skills | `LearningSkillsScreen.tsx` | Implemented | Grid editing + report model. |
| Loaned Items | `LoanedItemsScreen.tsx` | Implemented | Class/student item mapping and edits. |
| Device mappings / iPad IDs | `DeviceMappingsScreen.tsx` | Implemented | Per-student code mapping. |

## Reports and Export

| Legacy Area | Desktop Equivalent | Status | Notes |
| --- | --- | --- | --- |
| Class/Student/Category reports | `ReportsScreen.tsx` + sidecar report models | Implemented | Filters and PDF export via Chromium pipeline. |
| Attendance report | `reports.attendanceMonthlyModel` | Implemented | Export wired and E2E-covered. |
| Learning skills report | `reports.learningSkillsSummaryModel` | Implemented | Export wired and E2E-covered. |
| Backup | ZIP bundle export/import | Implemented | Sidecar staged and packaged smoke tests present. |
| Exchange CSV | Export/import flow | Implemented | File picker assisted. |

## Deferred (Current Track)

| Legacy Area | Status | Reason |
| --- | --- | --- |
| Planner integration | Deferred | Not classroom-critical for parity-first delivery. |
| Email setup/send workflows | Deferred | Peripheral integration postponed. |
| Pixel-perfect VB6 visual clone | Deferred | Functional parity prioritized first. |
