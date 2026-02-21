# Legacy-Truth Evidence Lane

## Two-lane model

### Regression lane (always required)
- Purpose: catch drift early on every PR.
- Command: `bun run test:parity:regression`
- Fixtures: `calc-behavior-locks.json`, `final-marks.json`
- CI: always required.

### Strict truth lane (conditional until artifacts are ready)
- Purpose: verify strict parity against fresh legacy outputs.
- Command: `bun run test:parity:strict`
- Readiness check: `bun run test:parity:truth` and `bun run test:parity:status:json`
- CI: auto-required when manifest flips `strictReady` to `true`.

## Strict-ready playbook

### Plain-language checklist (what we need from the legacy app)

To prove math parity against legacy truth, we need a **freshly recalculated legacy class** and the exact output files below.

1. Open legacy MarkBook and load class folder `Sample25/MB8D25`.
2. Open each mark set (`MAT1`, `MAT2`, `MAT3`, `SNC1`, `SNC2`, `SNC3`), run Calculate/Recalculate, then Save.
3. Copy out:
   - one fresh final-marks export JSON (`fresh-final-marks.json`)
   - six refreshed mark files (`MAT18D.Y25`, `MAT28D.Y25`, `MAT38D.Y25`, `SNC18D.Y25`, `SNC28D.Y25`, `SNC38D.Y25`)
4. Place those files into the strict fixture paths listed below.
5. Run checksum command and update manifest checksums.

If those files are present and checksums match, strict parity can be turned on with `strictReady=true` in the manifest.

1. Put fresh legacy files in these exact paths:
   - `fixtures/legacy/Sample25/expected/fresh-final-marks.json`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT18D.Y25`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT28D.Y25`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT38D.Y25`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC18D.Y25`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC28D.Y25`
   - `fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC38D.Y25`
2. Generate SHA-256 checksums from repo root:
   ```bash
   shasum -a 256 \
     fixtures/legacy/Sample25/expected/fresh-final-marks.json \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT18D.Y25 \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT28D.Y25 \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/MAT38D.Y25 \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC18D.Y25 \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC28D.Y25 \
     fixtures/legacy/Sample25/expected/fresh-markfiles/MB8D25/SNC38D.Y25
   ```
3. Update `fixtures/legacy/Sample25/expected/parity-manifest.json`:
   - add/update checksum entries under `checksums`
   - set `strictReady` to `true`
4. Verify preflight locally:
   - `bun run test:parity:status`
   - `bun run test:parity:truth`
   - `bun run test:parity:strict`
5. Commit fixtures + manifest together in one PR.

## Machine-readable status contract

- `bun run test:parity:status:json` emits manifest + lane readiness payload.
- `bun run test:parity:truth` emits one JSON payload with:
  - `mode: "ready" | "not-ready" | "checksum-mismatch" | "schema-mismatch"`
  - `strictRequiredByManifest: boolean`
  - `missing[]`
  - `checksumMismatches[]`
  - `suitesRan` and `suitesExitCode`

## Rules
- Keep strict tests committed even when strict artifacts are missing.
- Never loosen regression-lock tests to hide drift.
- Do not flip `strictReady` until all strict files and checksums are present.
