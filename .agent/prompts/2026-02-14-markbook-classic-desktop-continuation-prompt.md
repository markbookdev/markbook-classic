# 2026-02-14 - Continuation Prompt: MarkBook Classic Desktop (Electron + Bun + Rust)

You are working in this repo:
- `/Users/davemercier/dev/markbook/markbook-classic`

This project is the modern offline desktop rewrite of the legacy MarkBook Windows VB6 application.

## 0) Read These First (Project Continuity)
This repo already contains a full archive of work done so far and the ADRs:
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/conversations/2026-02-14-desktop-bootstrap-electron-bun-rust.md`
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/tasks/v1-desktop-bootstrap-and-legacy-import.md`
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/decisions/2026-02-14-desktop-stack-electron-bun-rust-sidecar.md`
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/product/markbook-classic-desktop.md`
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/important/2026-02-14-legacy-data-and-import-slice.md`

Also note: there is an older bootstrap attempt folder that should be ignored (unless explicitly requested):
- `/Users/davemercier/dev/markbook/markbook-desktop`

## 1) Where the Legacy MarkBook Source Lives (VB6 "Source of Truth")
Legacy repository folder (contains VB6 source, manuals, sample data, and XP-era build artifacts):
- `/Users/davemercier/dev/markbook/markbook-windows-classic`

Key legacy documentation and build notes:
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook Win - Year to Year - 2025.docx`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook Win - Year to Year - 2025.pdf`

Primary VB6 source folder for MarkBook V12 (2025):
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025`

Other major legacy program folders (context only, not yet ported in the new app):
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_v11 - 2025`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MBTrans 2025`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/CFG_2025`

VB6 project entrypoint (V12 2025):
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/MB_V12 2025.VBP`
  - Important observed settings in the `.VBP`:
    - `ExeName32="MkBk2025.exe"`
    - `Startup="frmAbout"` (splash/license gate)

Main legacy module (monolithic globals + file IO + calculations):
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/MB_V12.BAS`
  - Has constants like:
    - `Ver$ = "Version 12.5.20"`
    - `PrgYear$ = "2025"`
    - `AcademicYear$ = "2025/2026"`
  - Contains extensive `Open ... For Input/Output` usage for legacy file formats and class folder files.
  - Contains references suggesting offline licensing / gating:
    - `C:\\gnupg\\gpg.exe`
    - `C:\\gnupg\\markbook.lic`
    - year-specific `MkBk2025.DLL` behavior described in the Year-to-Year doc.

Key VB6 forms (screens) in the `.VBP` include:
- `MAIN.FRM` (main screen; open class / open mark set)
- `CLLOAD.FRM` (create/open/import class)
- `CLEDIT.FRM` (class list editing)
- `MRKENTRY.FRM` (mark entry)
- `MARKSET.FRM` (mark set setup)
- `ATTEND.FRM` (attendance)
- `EVA.FRM`, `EVC.FRM`, `EVI.FRM` (analysis/reporting flows)
- `CLPRINT.FRM` (printing forms)
- `SEATPLAN.FRM` (seating plan)
- `KIDNOTES.FRM` (notes)
- `LS.frm` (learning skills)
- `ImportMarks.FRM`, `Export.frm`
- `SETUP.FRM` (includes Email settings per manual)
- `BACKUP.FRM`, `MKUPDATE.FRM`

Legacy manuals (good for UI/workflow understanding):
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Manuals/MarkBook Quick Start Guide.pdf`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Manuals/Using_Mark_Sets.pdf`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Manuals/MarkBook_Email.pdf`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Manuals/MarkBook_Class_Exchange.pdf`
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Manuals/Course_Description_Manual.pdf`

## 2) Legacy Data Reality (Observed from Sample Data)
Legacy MarkBook stores a class as a folder with many plaintext files, including year-coded extensions like `.Y25`.

In the legacy VB6 folder, sample data is at:
- `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/Sample25`

In THIS repo, the sample is copied for repeatable import/testing:
- `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25`

Example class folder:
- `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25/MB8D25`

Key file types we inspected (examples in MB8D25):
- Class list file:
  - `CL8D.Y25`
  - INI-like `[Section]` headers, then `[Class List]` section:
    - a student count line (e.g. `27`)
    - then comma-delimited student rows, e.g.:
      - `1 ,O'Shanter,Tam,M,005659,8D,...,20120209,111111`
- Mark set files:
  - `MAT18D.Y25`, `SNC18D.Y25`, etc.
  - Contains `[Categories]` and `[Marks]` sections; assessments appear as repeated blocks:
    - date line `YYYY MM DD`
    - category/strand name
    - assessment title
    - an integer flag
    - a summary line (multiple numeric fields)
    - then per-student mark lines
  - Sentinel values exist (example: `-10 ,-1` appears in marks data).
- Remarks per assessment:
  - `MAT18D.RMK` (appears to store per-student remark strings per assessment, lots of blanks)
- Types/flags per assessment:
  - `MAT18D.TYP` (observed as a list of integers, many zeros)
- Comment index:
  - `MAT18D.IDX` (references comment banks like `COMMENT.BNK`, has max chars)
- Student notes:
  - `8DNOTE.TXT`
- Seating plan:
  - `8D.SPL`
- Loaned items:
  - `MAT18D.TBK`
- iPad/device mapping:
  - `8D.ICC`
- HTML export config:
  - `HTML_8D_All.CFG`

## 3) What’s Already Implemented in This Repo (Do Not Rebuild)
This repo is already initialized as the Electron+Bun+Rust monorepo and includes a working dev loop.

Root:
- `/Users/davemercier/dev/markbook/markbook-classic/package.json` (Bun workspaces + scripts)
- `/Users/davemercier/dev/markbook/markbook-classic/CLAUDE.md` (filled guidance)
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/` (workflow docs + archives)

Desktop app:
- `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop`
  - Electron main spawns sidecar and proxies NDJSON requests:
    - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/electron/main.js`
    - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/electron/preload.js`
  - React renderer contains a placeholder grid and UI buttons:
    - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/src/renderer/ui/App.tsx`
  - Renderer can:
    - Select workspace folder
    - Import legacy class folder (currently imports class list only)

Sidecar:
- `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd`
  - SQLite file is created in the selected workspace folder as `markbook.sqlite3`.
  - IPC methods currently implemented:
    - `health`
    - `workspace.select`
    - `classes.list`
    - `class.importLegacy` (v0: finds and parses `CL*.Yxx` in a legacy class folder and imports class+students)

Fixture:
- `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25` (copied from the legacy folder)

How to run dev:
1. `cd /Users/davemercier/dev/markbook/markbook-classic`
2. `bun install`
3. `bun run sidecar:build:debug`
4. `bun run dev`

## 4) Immediate Next Milestone (Implement This Next)
Implement legacy import Phase 1 for Mark Sets, and wire real data into the grid.

### 4.1 SQLite schema additions (Rust sidecar)
Add tables (minimum):
- `mark_sets(id, class_id, code, description, weight_total, sort_order, ...)`
- `categories(id, mark_set_id, name, weight, sort_order, ...)`
- `assessments(id, mark_set_id, idx, date, category_name, title, weight, out_of, flags_json, ...)`
- `scores(id, assessment_id, student_id, raw_value, normalized_value, status, flags_json, ...)`

Important:
- Store original raw parsed lines in JSON columns for lossless round-trip during early development.
- Store "status" as an enum-like string for sentinel/missing states (do not overload numeric fields).

### 4.2 Legacy parsing (start with one file)
Start with:
- `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25/MB8D25/MAT18D.Y25`

Implement a parser that produces:
- mark set metadata (from file header if available, else infer from filename)
- categories and weights from `[Categories]`
- assessments from `[Marks]`
- score rows per student per assessment

Also plan for companion files for the same base name:
- `MAT18D.RMK` (assessment remarks)
- `MAT18D.TYP` (assessment flags)
- `MAT18D.IDX` (comment system metadata)

### 4.3 Sidecar methods to add (contract)
Add these IPC methods and wire them in Electron:
- `marksets.list` params `{ classId }`
- `markset.open` params `{ classId, markSetId }`
- `grid.get` params `{ classId, markSetId, rowStart, rowCount, colStart, colCount }`
- `grid.updateCell` params `{ classId, markSetId, row, col, value, editKind }`

Return payloads must be stable and should be defined/validated in:
- `/Users/davemercier/dev/markbook/markbook-classic/packages/schema`

### 4.4 Renderer changes (wire real grid)
Replace the placeholder grid in:
- `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/src/renderer/ui/App.tsx`

With:
- class selection state (choose a class from list)
- mark set list UI (choose a mark set)
- grid backed by `grid.get`
- edit handler calling `grid.updateCell`

Acceptance criteria:
- Import legacy class folder MB8D25:
  - class is created
  - students are imported
  - at least one mark set imported (MAT*)
  - assessments and scores exist in SQLite
- UI can open MAT* mark set and show a real grid of students x assessments.
- Editing a mark updates SQLite and is visible after refresh/reopen.

## 5) Reporting/PDF (Do Later, But Don’t Break the Path)
Reports are critical, but do not implement full reports before mark set import + real grid.

Keep the pipeline shape:
1. Renderer requests a report model from sidecar: `reports.renderModel` (not implemented yet).
2. HTML/CSS templates in `packages/reports`.
3. Electron main exports via `printToPDF` in a hidden BrowserWindow.

This is already partially scaffolded:
- `/Users/davemercier/dev/markbook/markbook-classic/packages/reports/src/index.ts` contains a stub renderer.
- `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/electron/main.js` contains an `exportPdfFromHtml` helper.

## 6) Security and Data Handling Requirements
- Renderer must not gain filesystem or SQLite access.
- All class data stays local; do not add any background network dependency.
- Never commit secrets or credentials to `.agent/*` or any repo file.

## 7) Additional Legacy Context (Optional, But Helpful)
If you need to map legacy UI and workflows to new screens, use:
- Quick Start Guide: class creation steps, main screens, printing, assessments.
- Using Mark Sets: semantics, limits (up to 100 mark sets), combined classes workflows.
- Email manual: SMTP flows (future), credential handling considerations.
- Class Exchange manual: MB Exchange CSV rules and constraints (future).

## 8) Output Expectations
When you finish the next milestone, update:
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/tasks/v1-desktop-bootstrap-and-legacy-import.md`
  - mark set import status
  - new IPC methods implemented
  - any parsing gotchas and sentinel mappings

If asked to archive, write an updated conversation summary under:
- `/Users/davemercier/dev/markbook/markbook-classic/.agent/conversations/`
and redact anything sensitive.

