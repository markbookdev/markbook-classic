# 2026-02-14 - Desktop Bootstrap (Electron + Bun + Rust)

## Summary
We oriented on the legacy MarkBook Windows VB6 codebase and file formats, selected a modern desktop stack (Electron + Bun + Rust sidecar + SQLite), and implemented an initial working skeleton in a new repo (`markbook-classic`) including an end-to-end legacy import slice for `CL*.Yxx` (class + students).

## What Was Discussed and Decided
- Product intent:
  - Rebuild the legacy MarkBook Windows offline app (no cloud dependency) with a path to macOS.
  - Use legacy manuals and sample data to understand screens/modules and validate parity.
- Stack decision:
  - Electron for cross-platform desktop shell (report/PDF heavy).
  - Bun as package manager and workspace manager.
  - Rust sidecar for privileged ops (SQLite, filesystem import/export) with NDJSON IPC.
- UX decisions:
  - Use a high-performance spreadsheet grid for marks entry; selected `@glideapps/glide-data-grid`.
- Report requirements:
  - Reports are very important; target is professional + consistent PDFs (not pixel-identical to legacy).

## Repo Initialization
The user asked to clone `https://github.com/markbookdev/markbook-classic` into `/Users/davemercier/dev/markbook/markbook-classic`. The repo was empty, so we initialized it with the desktop monorepo skeleton described above.

Note: A previously created folder `/Users/davemercier/dev/markbook/markbook-desktop` exists from earlier work; it was not deleted or modified after switching to the cloned repo path.

## What Was Done (Implementation)
### 1. Monorepo scaffolding (Bun workspaces)
- Root `package.json` defines workspaces `apps/*` and `packages/*`.
- Desktop app is in `apps/desktop` with:
  - Vite dev server for renderer
  - Electron main/preload
  - React renderer using `@glideapps/glide-data-grid` as a placeholder grid

### 2. Electron security boundaries
- `contextIsolation: true`, `nodeIntegration: false`, `sandbox: true`
- Renderer calls `window.markbook.*` via preload only.
- Sidecar is spawned by Electron main and accessed via NDJSON protocol over stdin/stdout.

### 3. Rust sidecar (`markbookd`)
Implemented:
- NDJSON request/response protocol:
  - `{id, method, params}` -> `{id, ok, result}` or `{id, ok, error}`
- SQLite bootstrap with tables:
  - `classes(id, name)`
  - `students(id, class_id, last_name, first_name, student_no, birth_date, active, raw_line)`
- Methods:
  - `health`
  - `workspace.select`
  - `classes.list`
  - `class.importLegacy` (v0: imports from `CL*.Yxx`)

### 4. Legacy fixture data
Copied sample legacy data into the new repo for repeatable import testing:
- `fixtures/legacy/Sample25`

### 5. Claude Code agent structure
Added `.agent/` folder structure and a `CLAUDE.md` with repo guidance and security rules.

## Current State of the Work
Working capabilities in dev:
- `bun install` succeeds.
- Rust sidecar builds (`bun run sidecar:build:debug`) after installing Rust toolchain (done locally via Homebrew).
- Desktop dev loop (`bun run dev`) launches the UI.
- In the UI:
  - Select Workspace: picks a folder; sidecar creates `markbook.sqlite3` there.
  - Import Legacy Class Folder: selects a class folder containing `CL*.Yxx`; sidecar imports class + students.
  - Classes list shows imported classes.

Not yet implemented:
- Import mark set assessment files (`*.Yxx` like `MAT18D.Y25`) and companions (`.RMK`, `.TYP`, `.IDX`, etc.).
- Real grid data wired via `grid.get` / `grid.updateCell`.
- Report model generation and full PDF export workflow (only PDF export helper exists; templates are stubs).
- MB Exchange CSV import/export.
- Packaging and bundling sidecar into `apps/desktop/resources/markbookd/` for production builds (placeholder dir exists).

## Decisions Made
- Electron chosen over Tauri primarily due to report/PDF reliability and ecosystem maturity.
- Bun chosen to align with the TS/Next.js web rewrite and to manage workspaces.
- Rust sidecar chosen for DB + filesystem + parsing reliability and to maintain a strong security boundary.
- Grid library chosen: `@glideapps/glide-data-grid`.

## Open Questions / Next Steps
- Confirm the long-term storage shape:
  - single workspace DB (current) vs one DB per class vs hybrid export bundles.
- Define mark/assessment semantics:
  - how to map legacy sentinel values (e.g. `-10,-1`) into typed states.
- Implement legacy import Phase 1:
  - parse `*.Yxx` mark set files into categories, assessments, scores.
- Implement typed IPC schemas in `packages/schema` and validate all IPC payloads.
- Implement report models and finalize HTML/CSS templates for PDFs.

## Context for Next Session
Start by implementing mark set import for a single file:
- Use the fixture at `fixtures/legacy/Sample25/MB8D25/MAT18D.Y25`.
- Create tables: `mark_sets`, `categories`, `assessments`, `scores`.
- Add sidecar methods:
  - `marksets.list`
  - `markset.open`
  - `grid.get`
  - `grid.updateCell`
- Wire the renderer to display a real mark set grid.

## Related Files
Root:
- `.gitignore` - Added optional `.agent/scratch` ignore and standard ignores.
- `CLAUDE.md` - Added repo guidance and agent workflow rules.
- `package.json` - Bun workspaces and scripts.
- `README.md` - Repo overview and dev commands.

Agent workflow:
- `.agent/README.md`
- `.agent/product/markbook-classic-desktop.md`
- `.agent/tasks/v1-desktop-bootstrap-and-legacy-import.md`
- `.agent/decisions/2026-02-14-desktop-stack-electron-bun-rust-sidecar.md`
- `.agent/important/2026-02-14-legacy-data-and-import-slice.md`

Desktop app:
- `apps/desktop/package.json`
- `apps/desktop/tsconfig.json`
- `apps/desktop/vite.config.ts`
- `apps/desktop/electron/main.js`
- `apps/desktop/electron/preload.js`
- `apps/desktop/src/renderer/index.html`
- `apps/desktop/src/renderer/main.tsx`
- `apps/desktop/src/renderer/global.d.ts`
- `apps/desktop/src/renderer/ui/App.tsx`
- `apps/desktop/resources/markbookd/.gitkeep`

Shared packages:
- `packages/schema/package.json`
- `packages/schema/src/index.ts`
- `packages/core/package.json`
- `packages/core/src/index.ts`
- `packages/reports/package.json`
- `packages/reports/src/index.ts`
- `packages/ui/package.json`
- `packages/ui/src/index.ts`

Rust sidecar:
- `rust/markbookd/Cargo.toml`
- `rust/markbookd/package.json`
- `rust/markbookd/src/main.rs`

Fixtures:
- `fixtures/legacy/Sample25/` (copied legacy sample data)

