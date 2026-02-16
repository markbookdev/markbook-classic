# Legacy Data and Import Slice (CL*.Yxx)

## Context
Legacy MarkBook Classic data is stored in a folder per class. The first import slice implemented for the rewrite reads the class list file (`CL*.Yxx`) to create a class and students in SQLite.

Fixture used during development:
- `fixtures/legacy/Sample25/MB8D25/CL8D.Y25`

## Insight
The `CL*.Yxx` format is INI-like with `[Section]` headers. It contains:
- metadata and settings
- Mark Set definitions
- `[General Information]` block with quoted strings
- `[Class List]` block with a count line, then comma-delimited student rows

Even in the sample, there are lots of empty `""` lines and older-version header strings. Parsers must be tolerant and should not assume all metadata is complete or modern.

## Why It Matters
Importing legacy data is the highest-risk part of parity. Starting with a small slice (`CL*.Yxx`) gave us:
- a validated end-to-end pipeline (UI -> IPC -> sidecar -> SQLite -> UI)
- a place to standardize error reporting early (file path, parse errors, missing sections)
- a DB schema shape for class and student identity

## Implementation Notes
- Sidecar method: `class.importLegacy` currently:
  - finds the first `CL*.Yxx` file in the selected legacy folder
  - parses `[General Information]` for class name (heuristic)
  - parses `[Class List]` for students (best-effort)
  - writes `classes` + `students` within a SQLite transaction

## Related Files
- `rust/markbookd/src/main.rs`
- `fixtures/legacy/Sample25/` (test data)

