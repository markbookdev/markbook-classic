# Final Mark Goldens (Manual Capture)

`final-marks.json` is used by `rust/markbookd/tests/calc_parity_goldens.rs` to lock per-student
final marks to the legacy app's results.

## How To Update / Expand

1. Open legacy MarkBook (Windows).
2. Open class folder: `fixtures/legacy/Sample25/MB8D25`
3. For a given mark set (e.g. MAT1):
   - Ensure Calculate/Recalculate has been run.
   - Read each student's **final/current mark** as displayed by MarkBook for that mark set.
4. Update `final-marks.json` with entries in the form:

```json
{
  "MAT1": {
    "Last, First": 83.4
  }
}
```

Notes:
- Use 1-decimal rounding as displayed by the legacy UI.
- Only include students that are valid for the mark set (legacy `valid_kid(kid, MkSet) == 1`).
  In Sample25, some students are inactive (`valid_kid(kid,0)=0`) and should not have a final mark.

