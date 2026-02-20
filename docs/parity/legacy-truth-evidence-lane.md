# Legacy-Truth Evidence Lane

## Two-Lane Model

## Regression lane (always-on)
- Purpose: detect drift quickly on every change.
- Inputs: committed fixture behavior locks and expected files.
- Gate: required on every merge.

## Strict legacy-truth lane (conditional)
- Purpose: assert exact match to fresh legacy outputs.
- Inputs: freshly generated legacy exports placed in expected strict paths.
- Gate: enabled as soon as fresh artifacts are available.

## Operational Rules
- Keep strict lane tests present in repo even when artifacts are missing.
- Strict preflight must report exact missing files and expected paths.
- Never weaken regression locks to mask true drift.
- Strict CI enforcement is controlled by `fixtures/legacy/Sample25/expected/parity-manifest.json`:
  - `strictReady: false` => strict lane optional/pending
  - `strictReady: true` => strict lane required in CI (`quality-gates.yml`)
