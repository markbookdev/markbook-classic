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
- Overall: IN PROGRESS (parity-focused beta)
- Bootstrap: DONE
- Sidecar/SQLite: DONE (core + companions + setup tables)
- Legacy import v0: DONE (CL file -> class + students)
- Mark set import: DONE (mark_sets/categories/assessments/scores)
- Grid backed by real data: DONE (Glide grid via grid.get + grid.updateCell + bulk/state ops)
- Reports pipeline: DONE (grid + summary PDFs via Chromium printToPDF)
- App shell + navigation: DONE (Dashboard, Marks, Students, Mark Set Setup, Attendance, Notes, Seating, Learning Skills, Reports, Backup, Exchange)
- Students screen: DONE (CRUD + active toggle + reorder)
- Mark Set Setup: DONE (categories + assessments CRUD/reorder + mark set settings)
- Companion import fidelity: IN PROGRESS (`.RMK/.TYP/.IDX/.R*` + `ALL!*.IDX` + `.TBK` + `.ICC` shipped)
- Calc endpoints: IN PROGRESS (`calc.assessmentStats`, `calc.markSetSummary` shipped; parity locks expanding)
- Playwright harness: DONE (17 specs green incl. attendance, seating, comments, bulk edit, backup/exchange, report exports)

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
   - finish strict VB6 parity extraction from `ipc` internals into `calc.rs` types/functions
   - expand locked fixtures to additional mark sets and category-level parity checks
2. IPC architecture hardening:
   - split `rust/markbookd/src/ipc/router.rs` into smaller handler modules (`ipc/handlers/*`)
   - keep transport/router thin and improve method-level unit tests
3. Marks/comments UX parity:
   - finish keyboard-first in-grid flows (copy/paste/fill/state controls already started)
   - tighten remarks/bank editing ergonomics and multi-student workflows
4. Reports completion:
   - attendance/class-list/learning-skills layouts are implemented; iterate on pagination and fidelity
   - add remaining parity report variants from legacy manuals
5. Packaging/release hardening:
   - validate production sidecar bundling on macOS + Windows
   - upgrade backup bundle format from sqlite-copy to zip manifest bundle

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
- TBK (`*.TBK`) is structured as: item count, per-item metadata, then per-student item-id/note rows.
- ICC (`*.ICC`) is a `(students+1) x (subjects+1)` matrix (row 0 defaults + per-student rows).
- Combined comment sets from `ALL!<class>.IDX` can overlap set numbers with mark-set IDX files; importer remaps to the next free set number per mark set to preserve both.
