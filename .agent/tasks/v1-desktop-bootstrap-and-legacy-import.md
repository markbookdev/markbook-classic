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
- Calc endpoints: IN PROGRESS (`calc.assessmentStats`, `calc.markSetSummary` shipped; calc-method routing now in `calc.rs`, golden set expanded to MAT2/SNC2)
- Playwright harness: DONE (30 specs green + 1 packaged smoke intentionally opt-in)
- IPC router de-monolith: DONE (`rust/markbookd/src/ipc/router.rs` is dispatch-only; no legacy fallback)
- Packaging hardening: IN PROGRESS (sidecar staging + packaged smoke + CI matrix added)

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
   - keep transport/router thin and improve method-level unit tests
   - continue deduping shared validation/SQL helpers across handlers
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

### Baseline / Regression Snapshot (2026-02-17)
- Rust sidecar tests: `cargo test --all-targets` => PASS
- Desktop renderer build: `bun run --cwd apps/desktop build:renderer` => PASS
- Playwright regression: `bun run test:e2e` => PASS (19/19)
- Purpose of snapshot:
  - lock known-good baseline before further parity extraction
  - ensure no IPC contract drift while moving calc/report handling into typed handlers

### Baseline / Regression Snapshot (2026-02-18)
- Rust sidecar tests: `cargo test --all-targets` => PASS
- Reports renderer tests: `bun run test:reports` => PASS
- Parity regression lane: `bun run test:parity:regression` => PASS
- Desktop E2E regression: `bun run test:e2e` => PASS (24 passed, 1 skipped packaged smoke)
- Packaging smoke: `bun run test:packaging` => PASS
- Packaged app smoke: `bun run test:e2e:packaged` => PASS
- Strict parity lane: `bun run test:parity:strict` => FAIL (expected until fresh legacy markfiles are added under `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/*.Y25`)
- New in this snapshot:
  - `grid.bulkUpdate` now returns optional rejection diagnostics (`rejected`, `errors[]`) without breaking the existing contract.
  - Marks screen now supports keyboard commit navigation (Enter/Tab/Shift+Tab) with single-click mark-cell editing.
  - Marks screen now includes an in-context remarks pane (comment set + bank quick insert + save) to avoid screen-hopping.
  - Added DB migration snapshot fixtures/tests to protect backward compatibility (`rust/markbookd/tests/db_migration_snapshots.rs`).
  - Added CI workflow for packaged smoke on macOS + Windows (`.github/workflows/packaged-smoke.yml`).

### Classroom Workflow Iteration Snapshot (2026-02-18, windowing/filters/bulk)
- Rust sidecar tests: `cargo test --all-targets` => PASS
- Reports renderer tests: `bun run test:reports` => PASS
- Playwright regression: `bun run test:e2e` => PASS (26 passed, 1 skipped packaged smoke)
- Packaging smoke: `bun run test:packaging` => PASS
- Packaged smoke: `bun run test:e2e:packaged` => PASS
- Parity regression lane: `bun run test:parity:regression` => PASS
- Implemented in this slice:
  - Marks grid now uses windowed/tiled loading in renderer state instead of eager full-matrix load on open.
  - Added `grid.get` range hard validation (`bad_params` on negative/oversized windows).
  - Added bulk membership endpoint `students.membership.bulkSet` and switched Students screen bulk actions to one IPC call.
  - Added single-remark endpoint `comments.remarks.upsertOne` and switched Marks remark save to point updates (no full set rewrite).
  - Report model endpoints now accept optional `filters` + `studentScope` and include applied values in model payloads.
  - Reports screen now exposes Marks-aligned filter controls (term/category/types/scope) and sends them into model requests.
  - Reports HTML templates now print applied filter/scope metadata in report headers.
  - Added E2E coverage:
    - `apps/desktop/e2e/marks-windowed-fetch.e2e.spec.cjs`
    - `apps/desktop/e2e/reports-filters-alignment.e2e.spec.cjs`
  - CI packaged smoke workflow now uploads diagnostics artifacts on failure (`test-results`, `playwright-report`, `apps/desktop/out`).

### Release Hardening Snapshot (2026-02-19, dual parity lane + packaged launch gate)
- Rust sidecar tests: `cargo test --all-targets` => PASS
- Reports tests: `bun run test:reports` => PASS
- Parity status lane: `bun run test:parity:status` => PASS (`regression` ready, `strict` pending expected fresh files)
- Parity regression lane: `bun run test:parity:regression` => PASS
- Desktop E2E regression: `bun run test:e2e` => PASS (27 passed, 1 skipped packaged smoke)
- Packaging smoke (bundle presence): `bun run test:packaging` => PASS
- Packaging smoke (real packaged launch): `bun run test:packaging:launch` => PASS
- Packaged E2E smoke: `bun run test:e2e:packaged` => PASS
- Implemented in this slice:
  - Added packaged launch smoke script: `apps/desktop/scripts/smoke-packaged-launch.cjs` and root script `test:packaging:launch`.
  - Added parity readiness helper script: `apps/desktop/scripts/parity-status.cjs` and root scripts `test:parity:status` + `test:parity:truth`.
  - Added strict-lane conditional test: `rust/markbookd/tests/final_marks_vs_fresh_legacy_exports.rs`.
  - Extended strict parity manifest requirements with fresh final marks file.
  - Added bulk update payload limit handling in sidecar (`grid.bulkUpdate` now returns optional `limitExceeded` diagnostics when oversized).
  - Added Rust load/limit lock test: `rust/markbookd/tests/grid_bulk_update_limits.rs`.
  - Expanded marks windowing E2E debug assertions (`tileCacheHits`, `tileCacheMisses`, `tileRequests`, `inflightMax`).
  - Added DB snapshot `v2` fixture and expanded migration integration coverage (`reports.markSetSummaryModel` read path + legacy import after migration).
  - Added CI workflow `quality-gates.yml` (Rust all-target, reports, parity regression, desktop E2E) and upgraded `packaged-smoke.yml` to run packaged launch smoke on both macOS and Windows.

### Router/Report Extraction Notes
- All `reports.*Model` methods are now handled directly in `rust/markbookd/src/ipc/handlers/reports.rs` (no legacy router fallback).
- Added Rust integration smoke test: `rust/markbookd/tests/reports_models_smoke.rs`.

### Calc Parity Notes
- Added assessment-stats recompute lock: `rust/markbookd/tests/assessment_stats_parity.rs`.
- Calc now models legacy `valid_kid(kid, MkSet)` semantics:
  - `students.active` corresponds to `valid_kid(kid, 0)`
  - `students.mark_set_mask` corresponds to the per-markset membership bits (`dummy$` trailing field in `CL*.Yxx`)
  - A student is considered valid for a mark set if `active && mask_bit(mark_set.sort_order)` (with `TBA` meaning include all mark sets).
- Added membership effect lock: `rust/markbookd/tests/valid_kid_membership_affects_averages.rs`.
- `*.Yxx` mark files include stored per-assessment average fields in summary lines, but those can drift if the class list validity flags change after a calculation pass. The lock test recomputes expected stats from raw scores + current valid-kid mask to match VB6 `Calculate` semantics.
- Added an optional strict lock against freshly-saved legacy mark files:
  - `rust/markbookd/tests/assessment_stats_vs_fresh_legacy_summaries.rs`
  - fixture instructions: `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/README.md`

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
- Legacy export helper files (`*.13`, `*.15`, `*.32`, `*.40`, `*.5`, `*.6`, `*.7`) may contain either 27 or 28 value rows per block for a 27-student class; parser must tolerate both.

### Legacy Screen Parity Deep-Scrub Snapshot (2026-02-19)
- Rust sidecar tests: `cargo test --all-targets` => PASS
- Reports tests: `bun run test:reports` => PASS
- Desktop E2E regression: `bun run test:e2e` => PASS (30 passed, 1 skipped packaged smoke)
- Parity regression lane: `bun run test:parity:regression` => PASS
- Packaging smoke: `bun run test:packaging` => PASS
- Packaged launch smoke: `bun run test:e2e:packaged` => PASS
- Implemented in this slice:
  - Added canonical parity backlog matrix: `.agent/tasks/legacy-desktop-parity-matrix.md`.
  - Added class wizard backend + persistence:
    - `class_meta` table/migration.
    - IPC: `classes.wizardDefaults`, `classes.createFromWizard`, `classes.meta.get`, `classes.meta.update`.
    - Class delete flow now removes `class_meta` rows.
  - Added class wizard UI screen:
    - `apps/desktop/src/renderer/ui/screens/ClassWizardScreen.tsx`.
    - App shell + dashboard navigation entry points.
  - Added mark set lifecycle backend support:
    - `mark_sets` columns: `is_default`, `deleted_at`, `block_title`.
    - IPC: `marksets.create`, `marksets.delete`, `marksets.undelete`, `marksets.setDefault`, `marksets.clone`.
    - `marksets.list` now supports `includeDeleted` and returns lifecycle metadata.
    - `markset.open` now guards against deleted mark sets.
  - Added mark set lifecycle UI:
    - Mark Set Manager panel (create/open/clone/default/delete/undelete) in `MarkSetSetupScreen`.
    - Block title editing in mark set settings.
  - Added legacy-style marks action strip functionality:
    - New Entry, Multiple New, Entry Update, Entry Heading, Weight, Multiple Update, Open Mark Set.
    - Backed by `assessments.create/update` and new `assessments.bulkCreate/bulkUpdate`.
  - Added legacy menu discoverability groupings in app shell (File/Mark Sets/Working On) with implemented vs pending actions.
  - Added/extended tests:
    - Rust: `class_wizard_meta.rs`, `markset_lifecycle.rs`.
    - Rust migration snapshot coverage expanded for new schema columns.
    - Playwright: `class-wizard.e2e.spec.cjs`, `markset-lifecycle.e2e.spec.cjs`, `marks-action-strip.e2e.spec.cjs`.

### Legacy-Parity Specset Snapshot (2026-02-20)
- Completed deep-scrub documentation pack based on `docs/MarkBook_Reference.pdf` + VB6 project/menu surfaces.
- Added full specset under:
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/00-source-index.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/01-reference-taxonomy-by-chapter.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/02-legacy-forms-and-menu-catalog.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/03-legacy-screen-contracts.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/04-legacy-file-and-data-semantics.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/05-vb6-calc-and-report-semantics.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/06-current-desktop-capability-map.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/07-gap-assessment-matrix.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/08-prd-classroom-core.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/09-prd-analytics-and-reports.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/10-prd-planner-and-publishing.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/11-prd-integrations-and-admin.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/12-nfr-release-and-operability.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/13-master-implementation-roadmap.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/14-chunked-backlog.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/parity-specset/15-traceability-matrix.md`
- Added curated parity summaries under:
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/parity/README.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/parity/gap-summary.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/parity/feature-status-dashboard.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/parity/legacy-truth-evidence-lane.md`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/parity/implementation-roadmap-summary.md`
- Validation: all required files from the parity specset plan exist and were cross-checked.

### Working-On Parity Closure Snapshot (2026-02-20)
- Focus: `EPIC-CORE-01` (legacy Working On clone/delete/hide/update-all semantics).
- Backend additions:
  - IPC: `entries.delete`, `entries.clone.save`, `entries.clone.peek`, `entries.clone.apply`.
  - IPC: `marks.pref.hideDeleted.get`, `marks.pref.hideDeleted.set`.
  - `assessments.list` now supports optional `hideDeleted` and returns optional `isDeletedLike`.
  - Deleted-like semantics now align with legacy intent:
    - `assessment.weight <= 0`, or
    - mark set `weightMethod = category` and category weight `<= 0`.
- Renderer additions:
  - Marks action strip now includes: Clone Entry, Load Clone, Delete Entry, Hide Deleted Entries toggle, Update All.
  - Marks grid now supports hidden-entry views while preserving underlying source-column mapping for windowed `grid.get`.
  - Added deterministic E2E hook: `window.__markbookTest.getMarksVisibleAssessments()`.
- New Rust tests:
  - `rust/markbookd/tests/entries_delete_semantics.rs`
  - `rust/markbookd/tests/entries_clone_roundtrip.rs`
  - `rust/markbookd/tests/assessments_hide_deleted_like.rs`
- New/expanded Playwright tests:
  - `apps/desktop/e2e/marks-action-strip.e2e.spec.cjs` (expanded)
  - `apps/desktop/e2e/marks-hide-deleted.e2e.spec.cjs` (new)
  - `apps/desktop/e2e/marks-update-all.e2e.spec.cjs` (new)
- Verification run for this slice:
  - `cargo test --test ipc_router_smoke` => PASS
  - `cargo test --test entries_delete_semantics` => PASS
  - `cargo test --test entries_clone_roundtrip` => PASS
  - `cargo test --test assessments_hide_deleted_like` => PASS
  - `bun x playwright test apps/desktop/e2e/marks-action-strip.e2e.spec.cjs apps/desktop/e2e/marks-hide-deleted.e2e.spec.cjs apps/desktop/e2e/marks-update-all.e2e.spec.cjs` => PASS
  - `bun run test:e2e` => PASS (`33 passed`, `1 skipped`)
  - `bun run test:parity:regression` => PASS
  - `bun run test:packaging` => PASS

### EPIC-CORE-02 Snapshot (2026-02-20, class update-from-legacy + markset transfer)
- Backend shipped:
  - `classes.legacyPreview` with deterministic match/new/ambiguous/local-only diagnostics.
  - `classes.updateFromLegacy` with locked defaults:
    - mode `upsert_preserve`
    - collisionPolicy `merge_existing`
    - preserveLocalValidity `true`
  - `marksets.transfer.preview` and `marksets.transfer.apply` with merge/append/stop collision handling.
  - `class_meta` import-link metadata persisted:
    - `legacy_folder_path`, `legacy_cl_file`, `legacy_year_token`, `last_imported_at`
- Renderer shipped:
  - Dashboard actions for preview/update from legacy folder.
  - Students screen import diagnostics panel (source/time/warning count).
  - Mark Set Setup transfer dialog (source class/set, preview collisions/alignment, apply policy).
- New Rust tests:
  - `rust/markbookd/tests/classes_update_from_legacy_upsert.rs`
  - `rust/markbookd/tests/classes_update_preserve_validity.rs`
  - `rust/markbookd/tests/classes_update_collision_policy.rs`
  - `rust/markbookd/tests/marksets_transfer_apply.rs`
  - `rust/markbookd/tests/db_class_meta_import_link_migration.rs`
- New/updated Playwright tests:
  - `apps/desktop/e2e/class-update-from-legacy.e2e.spec.cjs`
  - `apps/desktop/e2e/markset-transfer.e2e.spec.cjs`
  - extended `apps/desktop/e2e/students-membership.e2e.spec.cjs` to lock update-from-legacy validity preservation.
- Verification for this slice:
  - `cargo test --test classes_update_from_legacy_upsert --test classes_update_preserve_validity --test classes_update_collision_policy --test marksets_transfer_apply --test db_class_meta_import_link_migration` => PASS
  - `bun x playwright test apps/desktop/e2e/class-update-from-legacy.e2e.spec.cjs apps/desktop/e2e/markset-transfer.e2e.spec.cjs apps/desktop/e2e/students-membership.e2e.spec.cjs` => PASS
