# 07 - Gap Assessment Matrix

## Purpose
Rank parity gaps with a repeatable scoring system and convert them into implementation order.

## Scoring Method
- Functional parity gap: `0-5`
- Workflow parity gap: `0-5`
- Data parity gap: `0-5`
- Calculation impact: `0-5`
- Classroom criticality: `0-5`
- Release risk: `0-5`
- Implementation complexity: `0-5`

Priority index formula:
- `priority = (criticality*2 + functional_gap + calc_impact + release_risk) - complexity`

Status rules:
- Implemented = feature complete + tests + acceptance.
- Partial = usable but core parity path missing.
- Missing = no usable path.
- Deferred = explicitly out-of-wave.

## Matrix
| ID | Legacy area | Current status | Func gap | Workflow gap | Data gap | Calc impact | Criticality | Risk | Complexity | Priority | Notes |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| GAP-001 | Planner + course description (ch6/tools) | Missing | 5 | 5 | 3 | 1 | 4 | 3 | 4 | 13 | Entire module family absent. |
| GAP-002 | Class interactive analytics tabs (ch9) | Partial (read-only tab shipped) | 3 | 3 | 2 | 5 | 5 | 4 | 4 | 18 | Class analytics screen + backend model shipped; interactive deep actions still pending. |
| GAP-003 | Student analytics tabs (ch11) | Partial (read-only tab shipped) | 3 | 3 | 2 | 5 | 4 | 4 | 4 | 16 | Student analytics screen shipped; compare/trend/combined actions pending. |
| GAP-004 | Combined report analytics (ch12) | Implemented (read-only) | 2 | 2 | 1 | 5 | 4 | 3 | 3 | 15 | Combined analytics screen + combined report model/export shipped; write actions intentionally deferred. |
| GAP-005 | Comments transfer-mode UX parity | Implemented (core) | 1 | 1 | 1 | 2 | 5 | 2 | 3 | 11 | Compare/import/flood-fill transfer flows shipped with fit/max-length enforcement and diagnostics. |
| GAP-006 | Working On menu full parity | Implemented | 1 | 1 | 0 | 1 | 5 | 1 | 2 | 11 | Clone/delete/hide/update-all shipped (`entries.*` + marks UI + tests). |
| GAP-007 | Class update-from-file/SIS deep parity | Partial (core shipped) | 1 | 1 | 2 | 1 | 4 | 2 | 2 | 10 | `classes.legacyPreview`, `classes.updateFromLegacy`, and mark set transfer shipped; SIS/attach nuances remain. |
| GAP-008 | Setup subdomains (analysis/calc/comments/etc.) | Partial | 4 | 3 | 2 | 4 | 3 | 3 | 4 | 12 | scattered settings not fully exposed. |
| GAP-009 | Bulk email workflow | Missing | 4 | 3 | 2 | 1 | 2 | 3 | 3 | 9 | deferred until core parity closure. |
| GAP-010 | Chapter 13 external adapters breadth | Partial | 4 | 3 | 3 | 1 | 2 | 3 | 5 | 7 | focus on SIS/class exchange first. |
| GAP-011 | Visual/menu discoverability parity | Partial | 3 | 4 | 0 | 0 | 3 | 2 | 2 | 9 | final wave after functional closure. |
| GAP-012 | Legacy-truth strict evidence lane fully populated | Partial | 2 | 1 | 3 | 5 | 4 | 5 | 2 | 18 | blocked on fresh outputs; infra ready. |

## Priority Ordering (high to low)
1. GAP-002
2. GAP-012
3. GAP-003
4. GAP-001
5. GAP-008
6. GAP-004
7. GAP-006
8. GAP-007
9. GAP-005
10. GAP-009
11. GAP-011
12. GAP-010

## Interpretation
- Core-first classroom lane centers GAP-002/003/004/006/005 before planner/integrations.
- GAP-012 runs in parallel as evidence infrastructure and strict-goldens activation path.
- GAP-001 (planner) is strategic but not day-one classroom-critical for grading/reporting parity.
