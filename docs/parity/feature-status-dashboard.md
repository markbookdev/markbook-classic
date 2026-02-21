# Feature Status Dashboard

| Domain | Status | Notes |
| --- | --- | --- |
| Class lifecycle | Implemented (core) | wizard/profile/update-from-legacy plus attach legacy folder and re-import-attached workflows are shipped; edge-case depth still tracked in parity backlog |
| Students | Implemented | CRUD/reorder/active/membership complete |
| Mark sets | Implemented (core) | lifecycle manager + default/delete/undelete/clone + transfer preview/apply are shipped; niche legacy edge tooling remains backlog-only |
| Marks workflow | Implemented (core) | fast editing + Working On semantics (clone/delete/hide/update-all) and deterministic parity behavior are shipped |
| Attendance | Implemented | monthly matrix + reports |
| Seating | Implemented | assignments/blocked/auto-place |
| Notes | Implemented | per-student notes |
| Comments/Banks | Implemented (core) | baseline plus compare/import/flood-fill transfer-mode flows with fit constraints |
| Learning Skills | Implemented | edit + report model/export |
| Loaned Items | Implemented | dedicated UI + APIs |
| Device mappings | Implemented | dedicated UI + APIs |
| Analytics screens | Implemented (interactive read-only) | class + student + combined analytics shipped; class rows/drilldown and student compare/trend are now interactive with report handoff |
| Setup/Admin defaults | Partial (depth-expanded) | setup now includes integrations/marks/exchange/analytics plus attendance/comments/reports/security/printer defaults; full legacy setup breadth still pending |
| Planner/Course Description | Implemented (core depth) | units/lessons/publish + clone/copy-forward/bulk assign + course generation options and report exports are shipped |
| Backup/Restore | Partial | solid baseline, broaden parity options |
| Exchange/Integrations | Partial (Tier-A shipped) | class exchange + SIS preview/apply/export + admin transfer package preview/apply/export shipped; broader chapter-13 adapters pending |
| Packaging/Release hardening | Implemented | launch smoke and CI gates in place |
| Strict legacy-truth parity lane | Partial (strict-ready) | manifest/checksum/schema validation + CI truth-readiness wiring + playbook are shipped; fresh outputs are still required to flip `strictReady=true` |
