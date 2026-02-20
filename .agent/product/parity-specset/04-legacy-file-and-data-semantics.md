# 04 - Legacy File and Data Semantics

## Purpose
Canonical legacy file semantics for import/export fidelity.

## Class and Student Core Files
- `CL*.Yxx`
  - Class metadata, student roster, mark set definitions, membership mask bits.
  - Critical semantics:
    - student order is canonical source of `sort_order`.
    - trailing membership mask (`111111`, `000000`, `TBA`) drives `valid_kid` per mark set.

## Mark Set Core Files
- `*.Yxx` (e.g., `MAT18D.Y25`)
  - Categories, assessments, summary fields, per-student `(percent, raw)` pairs.
  - Sentinel mapping:
    - raw > 0 => `scored`
    - raw == 0 => `no_mark` (blank in grid, excluded from denominator)
    - raw < 0 (usually `-1`) => `zero` (display `0`, included as zero)

## Mark Set Companion Files
- `.RMK`
  - remarks data linked by student/entry contexts.
- `.TYP`
  - assessment type flags (summative/formative/diagnostic/self/peer patterning).
- `.IDX` and `.R*`
  - comment set index metadata and per-set remarks.
  - includes mark-set-specific files and combined `ALL!*.IDX` scenarios.
- `.BNK`
  - comment bank definitions and entries.

## Other Legacy Companion Files
- `.ATN`
  - attendance matrix and day-type data.
- `.ATT`
  - alternate attendance representation (fallback path if needed).
- `.SPL`
  - seating grid dimensions, blocked seats, assignment coding.
- `.NOTE.TXT`
  - per-student notes.
- `.TBK`
  - loaned items and issue state.
- `.ICC`
  - individual course/device code matrix.

## Legacy Export Artifacts Used for Parity Evidence
- `*.13`, `*.32`, `*.40`, `*.5`, `*.6`, `*.7`, `*.15`
  - used for behavior locks and final-mark regression checks.
- Caveat:
  - these are regression anchors unless fresh legacy-truth outputs are supplied.

## SQLite Representation Principles
- Preserve raw imported meaning first; derive display/aggregate meaning second.
- Keep additive migration strategy; never break existing workspace files.
- Keep explicit status enums where sentinel states matter (`scores.status`).

## Mapping Table (Legacy -> SQLite)
| Legacy concept | SQLite location | Notes |
| --- | --- | --- |
| Student roster order | `students.sort_order` | canonical row mapping |
| Active flag | `students.active` | `valid_kid(k,0)` analogue |
| Membership mask | `students.mark_set_mask` | bit by `mark_sets.sort_order`, `TBA` include-all |
| Mark set list | `mark_sets` | includes lifecycle flags |
| Category definitions | `categories` | weighted and ordered |
| Assessment definitions | `assessments` | index, type, term, weight, out_of |
| Raw score state | `scores.raw_value` + `scores.status` | `scored/no_mark/zero` |
| Remarks | `scores.remark` + comment tables | both entry-level and set-level remarks |
| Attendance | `attendance_*` tables | monthly and per-student month strings |
| Seating | `seating_*` tables | assignments and blocked mask |
| Loaned items | `loaned_items` | class/student scoped |
| Device mappings | `student_device_map` | class/student scoped |
| Notes | `student_notes` | per class/student |

## Import Error Contract Requirements
- Structured error code with details object.
- Include source location where possible:
  - `folder`, `filename`, `line`.
- Import should be transactional per class where feasible.
- Warn (do not hard fail) on optional companion file absence unless required for selected import mode.

## Compatibility Defaults
- Missing/short/invalid membership masks fail-open to include (`TBA` semantics) until explicitly edited.
- Inactive students remain visible by default across screens/reports unless scoped filter says otherwise.
- Keep old backup import compatibility while defaulting to current bundle format.
