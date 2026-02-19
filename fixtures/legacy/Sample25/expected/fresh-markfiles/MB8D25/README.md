# Fresh Mark File Goldens (Manual Export)

These files are used to lock strict parity against legacy MarkBook's per-assessment summary lines
stored inside the `*.Y25` mark files.

## Why

The summary line values (`avg_raw`, `avg_percent`) inside `*.Y25` can become stale if the class-list
validity flags (`valid_kid`) change after a calculation pass. To use them as a trustworthy parity
anchor, we need "fresh" mark files saved after a Calculate/Recalculate run in the legacy app.

## How To Generate

1. Open legacy MarkBook (Windows).
2. Open class folder: `fixtures/legacy/Sample25/MB8D25`
3. For each mark set MAT1..SNC3:
   - Open the mark set.
   - Run Calculate/Recalculate (whatever the UI calls it).
   - Save (this updates the `*.Y25` summary lines).
4. Copy these exact files into this folder:
   - `MAT18D.Y25`
   - `MAT28D.Y25`
   - `MAT38D.Y25`
   - `SNC18D.Y25`
   - `SNC28D.Y25`
   - `SNC38D.Y25`

After committing those files, enable/adjust the strict parity test:
`rust/markbookd/tests/assessment_stats_vs_fresh_legacy_summaries.rs`.

## Optional Strict Final-Mark Lock

To enable strict final-mark parity as well, export/transcribe the fresh legacy final marks into:

- `fixtures/legacy/Sample25/expected/fresh-final-marks.json`

using the same shape as `fixtures/legacy/Sample25/expected/final-marks.json`.

That file is consumed by:

- `rust/markbookd/tests/final_marks_vs_fresh_legacy_exports.rs`
