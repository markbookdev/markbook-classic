# Decision: Desktop Stack (Electron + Bun + Rust Sidecar)

Status: Accepted

## Context
We are rewriting the legacy MarkBook Classic desktop app (VB6/XP-era) as a modern offline desktop application with a path to macOS. The web version is also being rewritten using Next.js + TypeScript + shadcn/ui, so code reuse is a strong preference.

The app is:
- table-heavy (spreadsheet-like marks entry)
- report/PDF heavy (printing is critical)
- offline-first
- needs reliable import of legacy on-disk class data

## Options Considered
- Electron + TS/React renderer + native core (Rust) sidecar
- Tauri + TS/React renderer + Rust core
- .NET desktop (WPF/WinUI/Avalonia)
- Qt/QML with Rust core

## Decision
Use Electron for the desktop shell, Bun for package management and workspaces, and a Rust sidecar (`markbookd`) for local persistence and filesystem operations.

## Rationale
- Electron provides consistent HTML/CSS rendering and generally the lowest-risk path to stable, professional PDFs across Windows and macOS (Chromium-based).
- Bun aligns with the TS stack used in the web rewrite and improves dev ergonomics and workspace management.
- Rust sidecar:
  - isolates privileged filesystem and database code
  - provides strong parsing correctness and reliability options for legacy file import
  - enables a clear security boundary (renderer has no direct file/db access)

## Consequences
- App size and RAM footprint likely larger than Tauri due to Chromium.
- We must maintain an IPC protocol between renderer/main and sidecar.
- Packaging requires bundling a platform-specific sidecar binary.
- Rust toolchain becomes a build dependency (for dev and CI).

