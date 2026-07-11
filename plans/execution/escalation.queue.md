# APS Escalation Queue

Parked decisions from unattended APS-loop runs. Clear top-to-bottom; each item
is one closed question with a marked default. Move cleared items to
`## Resolved` with the decision on the first line.

## Open

### ESC-001 · verification-blocked · HIGH

- work item: JOURNEY-005 "three-platform release journey rehearsal" (left at:
  Linux leg complete + cross-platform CI dispatched; macOS/Windows interactive
  legs unrunnable from this host)
- blocking-since: 2026-07-11T17:45Z
- what I tried (≤3):
  - full Linux rehearsal on candidate `d6d3aa39c`
    ([record](../audits/2026-07-12-journey-005-linux-rehearsal.md)) — all
    eight journeys pass
  - dispatched `ci-nightly.yml` (run 29161637384) and `rust.yml` (run
    29161638249) on the candidate for automated cross-platform evidence
  - no macOS/Windows machine is reachable from this session for the
    interactive TTY/consent/reboot journeys
- evidence: `plans/audits/2026-07-12-journey-005-linux-rehearsal.md`; the two
  workflow runs above
- THE DECISION I NEED: Accept CI matrix runs as sufficient macOS/Windows
  evidence for this cut, or run the manual rehearsals yourself?
  - default: **manual** (JOURNEY-005 stays In Progress until you record the
    macOS + Windows interactive legs; the module's Expected Outcome asks for
    them)
  - alt: accept-CI (flip JOURNEY-005 Merged on green matrix runs alone —
    weakens the outcome to build/test evidence)
- if you do nothing: JOURNEY-005 (and therefore JOURNEY-006 and the cut)
  stays open; everything merged so far still ships whenever the cut happens
- resume token: phase=JOURNEY-005 sha=d6d3aa39c branch=main
- confidence: high

### ESC-002 · merge-approval · HIGH

- work item: JOURNEY-006 "outcome-based release gate" (left at: evidence
  matrix assembled; awaiting the operator's cut decision)
- blocking-since: 2026-07-11T17:45Z
- what I tried (≤3):
  - landed and council-verified the whole release cut (CIB-184 #3279, WOW-005
    #3280, CIB-073 #3282, CIB-183 #3283, ACTTUI-012 #3284, CIB-190 #3286)
  - verified JOURNEY-001..004 validation suites green on the candidate SHA
  - recorded the Linux outcome metrics (time-to-value 0.34 s first run /
    0.026 s repeat, one-next-action compliance, byte-stable contracts,
    redaction green)
- evidence: `plans/audits/2026-07-12-journey-005-linux-rehearsal.md`
  (metrics table); module statuses in
  `plans/modules/release-user-journeys.aps.md`
- THE DECISION I NEED: Approve cutting v0.9.0-beta from `d6d3aa39c` (or a
  later main tip) once ESC-001 is resolved?
  - default: **defer** (nothing is tagged; the release follows the `release`
    skill runbook when you approve)
  - alt: approve (I run the release skill's deterministic steps at the next
    unattended window)
- if you do nothing: no release is tagged; JOURNEY stays In Progress with all
  code merged
- resume token: phase=JOURNEY-006 sha=d6d3aa39c branch=main
- confidence: high

### ESC-003 · design-signoff · MEDIUM

- work item: ADR-105 (GBASE base+delta persistence) — noted in passing, not a
  JOURNEY item
- blocking-since: 2026-07-11T13:30Z
- what I tried (≤3):
  - all GBASE council decisions are locked and the extraction PR chain merged
    (#3266, #3267)
  - the ADR text still reads `Proposed`
- evidence: `plans/decisions/DECISION-LOG.md` ADR-105 row
- THE DECISION I NEED: Flip ADR-105 to Accepted?
  - default: **defer** (stays Proposed; no behavioural impact)
  - alt: accept (I update the ADR + DECISION-LOG in a bookkeeping PR)
- if you do nothing: the decision record lags the shipped implementation
- resume token: phase=LEARN sha=d6d3aa39c
- confidence: medium

## Resolved

(none yet)
