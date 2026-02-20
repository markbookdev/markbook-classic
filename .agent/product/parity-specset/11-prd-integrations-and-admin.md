# 11 - PRD: Integrations and Admin

## Product Goal
Close priority integration and setup/admin parity gaps while keeping classroom core stable.

## Scope Tiers

## Tier A (near-term)
- SIS/class exchange parity hardening.
- Setup domains critical to grading/reporting correctness.
- Admin transfer paths required for practical deployments.

## Tier B (later)
- TI-Navigator, eInstruction, TurningPoint.
- Additional legacy adapter targets not required for core classroom lane.

## Functional Areas
- Setup domains:
  - analysis/reporting options
  - attendance options
  - calculation/remarks
  - comments
  - dates/birthdays
  - email settings
  - learning skills settings
  - letter styles
  - password
  - printer options
- Integration domains:
  - SIS export/import variants
  - admin edition transfer files
  - class/mark-set transfer variants

## API Program (planned)
- `setup.analysis.*`, `setup.attendance.*`, `setup.calc.*`, `setup.comments.*`, `setup.email.*`, `setup.printer.*`, etc.
- `integrations.sis.*`, `integrations.adminTransfer.*`, adapter-specific endpoints.

## Constraints
- Additive only where possible.
- Preserve existing backup/exchange contracts.
- Keep integration failures isolated with clear diagnostics.

## Acceptance Criteria
- Setup options affecting calculations/reports are fully represented in sidecar and UI.
- Tier A integration workflows pass deterministic import/export tests.
