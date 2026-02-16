# Task: v1 Desktop Bootstrap and Legacy Import

## Goal
Stand up a working desktop skeleton (Electron + Bun + Rust sidecar) and implement the first slice of legacy import so we can iterate with real data early.

## Scope (This Task)
- Repo bootstrap (workspaces + Electron dev loop + Rust sidecar)
- IPC protocol plumbing (NDJSON)
- Workspace selection (user picks folder; sidecar creates/opens SQLite)
- `classes.list` + basic data persistence
- Legacy import v0: parse `CL*.Yxx` and import class + students
- Fixture data available in-repo for repeatable testing

## Status
- Overall: IN PROGRESS
- Bootstrap: DONE
- Sidecar/SQLite: DONE (classes + students tables)
- Legacy import v0: DONE (CL file -> class + students)
- Mark set import: DONE (mark_sets/categories/assessments/scores)
- Grid backed by real data: DONE (Glide grid via grid.get + grid.updateCell)
- Reports pipeline: DONE (grid + summary PDFs via Chromium printToPDF)
- App shell + navigation: DONE (Dashboard, Marks, Students, Mark Set Setup, Notes, Reports)
- Students screen: DONE (CRUD + active toggle + reorder)
- Mark Set Setup: DONE (categories + assessments CRUD/reorder + mark set settings)
- Calc endpoints: IN PROGRESS (`calc.assessmentStats`, `calc.markSetSummary` shipped)
- Playwright harness: DONE (edit persistence, reorder, grid PDF, summary PDF)

## Deliverables (Implemented)
- Desktop app launches in dev (`bun run dev`)
- Sidecar builds (`bun run sidecar:build:debug`)
- App can:
  - select a workspace folder
  - import a legacy class folder (CL + mark sets)
  - display imported class list (names)
  - select a mark set and view a real students x assessments grid
  - edit a cell and persist it to SQLite

## Next Steps (Prioritized)
1. Calc parity hardening:
   - port remaining VB6 weighting/filter semantics exactly (term/type/category edge cases, bonus handling)
   - add fixture parity assertions for final marks (named students) and median/rank behavior
2. Companion imports still missing:
   - parse/import `*.IDX` (comment bank metadata)
   - parse/import seating (`*.SPL`) and attendance companions
3. Remaining core screens:
   - Attendance (real data model + entry UI)
   - Seating Plan (import + editor + persistence)
4. Grid UX polish:
   - explicit “set zero” action (current edit semantics treat 0 as No Mark)
   - copy/paste, fill/bulk edits, keyboard parity
5. Reports expansion:
   - category analysis + student summary reports from `calc.markSetSummary`
   - pagination/fit improvements for wide classes
6. Packaging + data portability:
   - production sidecar bundling verification
   - backup/restore + exchange/export flows

## Notes
- Fixture data is currently copied into:
  - `fixtures/legacy/Sample25`

### Parsing Gotchas / Sentinel Mapping
- Mark files `*.Yxx` store per-student values as `percent, raw`.
- Legacy mark states:
  - `raw == 0` => No Mark (excluded from averages/weights, displayed blank)
  - `raw < 0` (typically `-1`) => Zero (counts as 0, displayed as 0)
- In SQLite `scores.status` is one of: `scored`, `no_mark`, `zero`.
- Percent is redundant for v1 grid; grid displays and edits raw score only.
