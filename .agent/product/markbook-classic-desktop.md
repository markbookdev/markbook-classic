# MarkBook Classic Desktop (Product)

## One-Liner
An offline-first, installable MarkBook Classic desktop app for teachers: class lists, mark sets, assessments, attendance, analysis, and professional report PDFs, with legacy data import.

## Background
The legacy MarkBook Windows app is a VB6 codebase historically built in a Windows XP VM, with class data stored primarily as plain text files in class folders (e.g. `CL*.Y25`, `MAT18D.Y25`, etc.). Users still want a local/offline app that does not require cloud connectivity.

This repo is the modern rewrite targeting Windows first with a path to macOS.

## Goals (P0)
- Run fully offline on modern Windows (and later macOS).
- Import legacy MarkBook class folders reliably.
- Provide fast spreadsheet-like mark entry UX (keyboard-first).
- Produce professional, consistent PDF reports.
- Keep data local and secure (no forced network).

## Non-Goals (Initial)
- Cloud sync or multi-user collaboration.
- Perfect pixel-identical report layouts vs legacy (target is professional + consistent).
- Reproducing every legacy integration and niche export in v1.

## Primary Users
- Classroom teacher (daily gradebook and attendance).
- Department head (exports and analysis).
- IT/admin (install, licensing, data storage locations).

## Core Use Cases (P0)
- Select a workspace (folder) that contains the local database and attachments.
- Create/open a class.
- Add/edit students (class list).
- Create/open a mark set; define categories/strands and weights.
- Enter/edit assessments and marks.
- Track attendance.
- Generate and export PDFs for reports.
- Backup/export class data.
- MB Exchange CSV import/export for cross-platform exchange (planned).

## Technical Summary (Implementation)
- Electron + React renderer (Bun-managed) provides UI.
- Rust sidecar owns:
  - SQLite persistence
  - legacy import (VB6 file formats)
  - exports/imports
  - report model generation
- Reports are rendered via HTML/CSS templates then exported to PDF by Chromium `printToPDF`.

## Data Model Notes
- v1: a single SQLite DB per workspace folder (`markbook.sqlite3`) plus attachments folder.
- Future: optional "portable class bundle" export for transfer/backup.

## Risks
- Legacy file formats have edge cases and corrupted user data in the wild.
- Calculation semantics and report outputs have subtle expectations.
- Printing/PDF layout stability requires early cross-platform testing.

