# 12 - NFR: Release and Operability

## Purpose
Define non-functional requirements and release gates for parity program delivery.

## Security Boundary (hard requirement)
- Renderer never directly touches filesystem or SQLite.
- Privileged operations only through preload/main/sidecar IPC.

## Reliability
- Sidecar launch must be verified in packaged app smoke checks.
- Import/backup/restore flows must return structured diagnostics and fail safely.

## Performance
- Marks grid must remain performant on larger classes via windowed fetch strategy.
- Bulk updates must have deterministic limits and diagnostics.

## Compatibility
- SQLite migrations must be additive and snapshot-tested.
- Existing workspaces must open and operate after upgrades.

## Testability
- Every parity chunk must include:
  - Rust tests (unit/integration)
  - Playwright E2E updates
  - parity-lane checks

## Release Gates (required)
- `cargo test --all-targets`
- `bun run test:reports`
- `bun run test:e2e`
- `bun run test:packaging`
- `bun run test:e2e:packaged`
- `bun run test:parity:regression`
- `bun run test:parity:strict` (conditional until strict fixtures provided)

## CI Requirements
- Quality gates workflow (core tests + e2e + parity regression).
- Packaged smoke workflow (macOS and Windows launch checks).
- Failure artifact upload for diagnosis (out bundles, logs, ready files, reports).

## Observability
- Maintain sidecar path/running diagnostics in packaged test mode.
- Keep parity status command to report strict-lane readiness.
