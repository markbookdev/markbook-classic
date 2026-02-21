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
| GAP-002 | Class interactive analytics tabs (ch9) | Implemented (interactive read-only) | 1 | 1 | 1 | 5 | 5 | 2 | 4 | 14 | Class analytics now includes server-side search/sort/paging, cohort filtering, assessment drilldown, and report handoff parity. |
| GAP-003 | Student analytics tabs (ch11) | Implemented (interactive read-only) | 1 | 1 | 1 | 5 | 4 | 2 | 4 | 12 | Student analytics now includes cohort compare and trend-across-marksets with read-only parity behavior. |
| GAP-004 | Combined report analytics (ch12) | Implemented (read-only) | 2 | 2 | 1 | 5 | 4 | 3 | 3 | 15 | Combined analytics screen + combined report model/export shipped; write actions intentionally deferred. |
| GAP-005 | Comments transfer-mode UX parity | Implemented (core) | 1 | 1 | 1 | 2 | 5 | 2 | 3 | 11 | Compare/import/flood-fill transfer flows shipped with fit/max-length enforcement and diagnostics. |
| GAP-006 | Working On menu full parity | Implemented | 1 | 1 | 0 | 1 | 5 | 1 | 2 | 11 | Clone/delete/hide/update-all shipped (`entries.*` + marks UI + tests). |
| GAP-007 | Class update-from-file/SIS deep parity | Partial (core+attach shipped) | 1 | 1 | 2 | 1 | 4 | 2 | 2 | 10 | `classes.legacyPreview`, `classes.updateFromLegacy`, `classes.importLink.get/set`, `classes.updateFromAttachedLegacy`, and mark set transfer are shipped; remaining nuances are edge-case attach/reimport ergonomics and SIS variants. |
| GAP-008 | Setup subdomains (analysis/calc/comments/etc.) | Partial (expanded) | 2 | 2 | 2 | 2 | 3 | 2 | 3 | 11 | Setup/Admin now includes integrations defaults, marks defaults, exchange defaults, analytics defaults, and analytics/report header defaults; full legacy setup breadth still pending. |
| GAP-009 | Bulk email workflow | Missing | 4 | 3 | 2 | 1 | 2 | 3 | 3 | 9 | deferred until core parity closure. |
| GAP-010 | Chapter 13 external adapters breadth | Partial (Tier-A shipped) | 2 | 2 | 2 | 1 | 2 | 2 | 4 | 7 | Tier-A CSV + admin transfer contracts/UI are shipped; broader adapter families remain deferred. |
| GAP-011 | Visual/menu discoverability parity | Partial (improved) | 2 | 3 | 0 | 0 | 3 | 2 | 2 | 8 | Legacy-style grouped actions now include Integrations and Planner alongside existing groups; full legacy action surface still pending. |
| GAP-012 | Legacy-truth strict evidence lane fully populated | Partial | 2 | 1 | 3 | 5 | 4 | 5 | 2 | 18 | blocked on fresh outputs; manifest/checksum preflight and CI strict-lane readiness controls are now shipped. |

## Priority Ordering (high to low)
1. GAP-012
2. GAP-004
3. GAP-002
4. GAP-001
5. GAP-003
6. GAP-005
7. GAP-006
8. GAP-008
9. GAP-007
10. GAP-009
11. GAP-011
12. GAP-010

## Interpretation
- Core-first classroom lane has closed GAP-002/003 to implemented (interactive read-only parity); remaining classroom emphasis is workflow breadth (planner/setup/integrations).
- Tier-A integrations and setup defaults now reduce operational risk for real-school import/export workflows without introducing IPC breaking changes.
- GAP-012 remains the highest-risk evidence lane item and continues as the strict-goldens activation path.
- GAP-001 (planner) is strategic but not day-one classroom-critical for grading/reporting parity.
