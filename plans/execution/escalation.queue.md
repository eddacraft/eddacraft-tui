# APS Escalation Queue

Parked decisions from unattended APS-loop runs. Clear top-to-bottom; each item
is one closed question with a marked default. Move cleared items to
`## Resolved` with the decision on the first line.

## Open

(none)

## Resolved

### ESC-001 · RESOLVED 2026-07-12: **accept-CI** (operator)

- decision: CI matrix evidence accepted in lieu of manual macOS/Windows
  interactive rehearsals for this cut.
- evidence at resolution: full `rust.yml` cross matrix GREEN on main — run
  29171614019 (post PR #3297, which fixed the macOS/APFS `base_store` claim
  races [CIB-194] and the Windows test harnesses; PR #3290 had cleared the
  accumulated non-unix dead-code drift). Linux interactive rehearsal:
  `plans/audits/2026-07-12-journey-005-linux-rehearsal.md`.
- consequence: JOURNEY-005 flipped Merged; the residual PR-CI cross-lint gap
  stays tracked as CIB-193 (Ready, non-blocking).

### ESC-002 · RESOLVED 2026-07-12: **approve** (operator)

- decision: v0.9.0-beta cut approved from the current main tip once ESC-001's
  evidence landed (it has). The release follows the `release` skill's
  deterministic steps.
- consequence: JOURNEY-006 gate cleared; release execution proceeds.

### ESC-003 · RESOLVED 2026-07-12: **accept** (operator) — no action needed

- decision: flip ADR-105 to Accepted.
- finding at resolution: already done — the GBASE close (default-on flip wave)
  set ADR-105 to `Accepted 2026-07-11 (operator, planning-council
  plan-89a47ac7)` in both the ADR and the DECISION-LOG row. Nothing to edit.

<details>
<summary>Original parked entries (audit trail)</summary>



</details>
