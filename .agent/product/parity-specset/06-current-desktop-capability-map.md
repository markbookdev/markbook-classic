# 06 - Current Desktop Capability Map

## Purpose
Snapshot of what the current desktop rewrite already supports end-to-end.

## Capability Inventory by Domain

## App shell and navigation
- Implemented screens in shell routing:
  - Dashboard, Class Wizard/Profile, Marks, Students, Mark Set Setup, Attendance, Notes, Seating Plan, Learning Skills, Loaned Items, Device Mappings, Calc Settings, Backup, Exchange, Reports.
- Legacy-style menu group presentation exists, but parity depth is mixed.
- Status: Partial.

## Workspace and sidecar lifecycle
- Workspace selection/open, sidecar health/version/meta, restart sidecar.
- Preferences and recent workspace handling in Electron/main.
- Status: Implemented.

## Classes and class metadata
- `classes.list/create/delete` plus wizard/defaults/meta APIs.
- Class profile edit and wizard create flows in UI.
- Status: Partial (legacy wizard breadth still narrower than VB6 CLLOAD).

## Students
- Full CRUD + reorder + active toggle.
- Mark-set membership matrix (single + bulk updates).
- Notes editing integrated.
- Status: Implemented.

## Mark sets
- Mark set list/open/import.
- Lifecycle operations create/clone/set-default/delete/undelete.
- Categories/assessments CRUD + reorder + bulk create/update.
- Mark set settings edits.
- Status: Partial (some legacy transfer/aux actions remain).

## Marks grid and entry workflow
- Windowed `grid.get` retrieval and caching.
- `grid.updateCell`, `grid.setState`, `grid.bulkUpdate` diagnostics.
- Keyboard commit navigation and bulk edit semantics.
- Results panel with calc filters.
- Status: Partial (legacy "Working On" actions still incomplete).

## Attendance
- Monthly open/edit, day type edits, bulk stamping.
- Attendance report model/export.
- Status: Implemented.

## Seating
- Seating get/save, blocked seats, assignments, auto-place options.
- Seating persistence and report linkage.
- Status: Implemented.

## Comments and remarks
- Comment sets CRUD/open/list.
- Bank CRUD/import/export and entry management.
- Marks-context remark upsert.
- Combined `ALL!*.IDX` merge support.
- Status: Partial (transfer-mode UX and compare/import nuances not fully surfaced).

## Learning skills
- Learning skills open/update/report model.
- Learning skills screen and report export.
- Status: Implemented.

## Loaned items and device mappings
- APIs and dedicated screens exist.
- Legacy companion import support exists.
- Status: Implemented.

## Backup and exchange
- Backup export/import, packaging staging, smoke tests.
- CSV exchange export/import.
- Status: Partial (legacy breadth of adapter targets not complete).

## Reports and export
- Mark set grid/summary, category analysis, student summary, attendance monthly, class list, learning skills summary.
- Filters + student scope alignment between marks and reports.
- Chromium PDF export pipeline wired.
- Status: Partial (full legacy analytics-screen parity is still missing).

## Calc parity
- Core calc endpoints and settings override APIs.
- valid_kid mask semantics integrated.
- behavior locks and strict-lane scaffolding in place.
- Status: Partial (fresh legacy-truth artifacts pending; advanced edge parity still in progress).

## Packaging and release hardening
- Sidecar staging for packaged app.
- packaged-dir and packaged-launch smoke scripts.
- CI gates for quality and packaged smoke.
- Status: Implemented for baseline readiness; cross-platform depth can still expand.

## Top-level IPC Method Families Present
- `health`, `workspace.select`, `calc.config.*`
- `classes.*`, `class.importLegacy`, `marksets.*`, `markset.*`
- `students.*`, `students.membership.*`, `notes.*`
- `categories.*`, `assessments.*`, `markset.settings.*`
- `grid.*`
- `attendance.*`, `seating.*`
- `comments.sets.*`, `comments.banks.*`, `comments.remarks.upsertOne`
- `loaned.*`, `devices.*`, `learningSkills.*`
- `reports.*Model`, `calc.*`
- `backup.*`, `exchange.*`

## Key Missing/Partial Areas Relative to Legacy
- Planner and course description modules.
- Full class/student/combined interactive analytics screens.
- Main-form operational depth for certain menu actions (clone/delete/hide/update-all completeness, transfer variants, setup subdomains).
- Broader chapter-13 integration adapters.
