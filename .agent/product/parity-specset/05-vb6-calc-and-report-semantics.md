# 05 - VB6 Calc and Report Semantics

## Purpose
Lock the semantics that determine mark/report parity and define evidence expectations.

## Core Semantic Rules

## valid_kid semantics
- Legacy: `valid_kid(k, j)` combines base active + mark-set membership bit.
- Desktop parity target:
  - valid for mark set = `students.active && mask_bit(mark_sets.sort_order)`
  - `TBA` = include all mark sets while active.

## Score-state semantics
- `no_mark`:
  - displayed blank.
  - excluded from denominators.
- `zero`:
  - displayed as `0`.
  - included in denominators as zero.
- `scored`:
  - displayed numeric raw value.

## Rounding
- VB6-style one-decimal `RoundOff` at legacy-equivalent boundaries.
- Avoid renderer-side ad-hoc rounding for parity-sensitive outputs.

## Weight and inclusion rules (entry/category/equal)
- Included-for-stats and included-for-final may differ.
- Weight-zero entries can appear in assessment stats while excluded from final mark calculations.
- Category-weight method fallback and bonus behavior must follow legacy algorithm paths.

## Calc methods
- 0: Average
- 1: Median
- 2: Mode
- 3: Blended Mode
- 4: Blended Median

## Mode-level configuration
- Load from `*_USR.CFG` where present.
- Support workspace override model in `workspace_settings`.
- Mode thresholds/symbols and `roff` must be part of applied settings diagnostics.

## Report model semantic contract
- Reports must consume sidecar calc/model outputs, not independent renderer math.
- Report filters (term/category/types/scope) must align with marks screen filters.
- Report headers must show applied filter metadata.

## Evidence Lanes

## Regression-lock lane (always-on)
- Uses fixture locks to detect drift quickly.
- Not considered legacy truth in absence of fresh exports.

## Legacy-truth lane (strict)
- Activates when fresh legacy exports are present.
- Fails hard on mismatch against authoritative outputs.

## Current parity risks to track
- Stale legacy summary lines vs current validity settings.
- Edge behavior for blended methods and mode tie-breaks.
- Transfer-mode comments and report-card fit interactions.

## Required test families
- `calc_behavior_locks` (broad drift checks).
- `calc_parity_goldens` (named-student locks).
- `assessment_stats_*` (per-assessment parity checks).
- strict fresh-summary and final-mark tests (conditional until fresh files supplied).
- report model alignment tests (filters and scope).

## Acceptance criteria for calc/report parity wave closure
- All report endpoints consume parity-locked calc contexts.
- Strict lane is executable without test rewrites when fresh artifacts are dropped in expected paths.
- Any change affecting score inclusion/rounding requires lock updates and explicit changelog note.
