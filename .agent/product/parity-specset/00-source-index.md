# 00 - Source Index

## Purpose
Canonical source inventory used for the full legacy-parity specset.

## Scope of Evidence
- Legacy product behavior and UX intent.
- Legacy data-file semantics and calculation semantics.
- Current desktop implementation status (UI, IPC, DB, tests, packaging).

## Primary Legacy Sources
- Manual (reference bible):
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/MarkBook_Reference.pdf`
- Manual supplements:
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/MarkBook_Math.pdf`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/Using_Mark_Sets.pdf`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/MarkBook_Class_Exchange.pdf`
  - `/Users/davemercier/dev/markbook/markbook-classic/docs/Course_Description_Manual.pdf`

## Primary Legacy Code Sources
- VB6 project root:
  - `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025`
- VB6 project file:
  - `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/MB_V12 2025.VBP`
- Primary legacy module:
  - `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/MB_V12.BAS`
- Main legacy shell/menu form:
  - `/Users/davemercier/dev/markbook/markbook-windows-classic/MarkBook on XP Folders and files 2025 09 18/MkBk_VB6/MarkBook/MkBk_V12 - 2025/MAIN.FRM`

## Primary Current-Desktop Sources
- Renderer app shell and screens:
  - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/src/renderer/ui/app/AppShell.tsx`
  - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/src/renderer/ui/screens/*.tsx`
- Electron boundary:
  - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/electron/main.js`
  - `/Users/davemercier/dev/markbook/markbook-classic/apps/desktop/electron/preload.js`
- Sidecar IPC router/handlers:
  - `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd/src/ipc/router.rs`
  - `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd/src/ipc/handlers/*.rs`
- Sidecar core:
  - `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd/src/db.rs`
  - `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd/src/legacy.rs`
  - `/Users/davemercier/dev/markbook/markbook-classic/rust/markbookd/src/calc.rs`
- Shared schemas:
  - `/Users/davemercier/dev/markbook/markbook-classic/packages/schema/src/index.ts`
- Reports templates:
  - `/Users/davemercier/dev/markbook/markbook-classic/packages/reports/src/index.ts`

## Fixture Sources
- Sample legacy class fixture:
  - `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25/MB8D25`
- Expected/parity fixtures:
  - `/Users/davemercier/dev/markbook/markbook-classic/fixtures/legacy/Sample25/expected`

## Existing Internal Tracking Docs
- Task tracker:
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/tasks/v1-desktop-bootstrap-and-legacy-import.md`
- Legacy parity matrix (prior pass):
  - `/Users/davemercier/dev/markbook/markbook-classic/.agent/tasks/legacy-desktop-parity-matrix.md`

## Evidence Extraction Notes
- Manual TOC extracted from pages 5-8 of `MarkBook_Reference.pdf`.
- Legacy menu tree extracted from `MAIN.FRM` menu declarations and captions.
- Legacy forms/modules/classes extracted from `.VBP` entries.
- Current capability inventory extracted from `AppShell.tsx` routes and sidecar `try_handle` method maps.

## Constraints Locked Across Specset
- Renderer never directly accesses filesystem or SQLite.
- Privileged flow remains `renderer -> preload -> Electron main -> sidecar`.
- Sidecar NDJSON IPC stays request/response with structured errors.
- Existing report export pipeline remains model -> HTML -> Chromium `printToPDF`.
- No breaking IPC changes in parity program; additive only unless explicitly versioned.
