<!--
APS Module: Continuous Improvement (RETIRED)
============================================
This module was a meta-theme without executable tasks. Its original concrete
intents mapped onto MAINT (codebase-maintenance); new standing intake now maps
onto CIB (continuous-improvement-backlog).
-->

# Continuous Improvement — RETIRED

| ID  | Owner | Status  |
| --- | ----- | ------- |
| CI  | —     | Retired |

**Superseded by:** [codebase-maintenance (MAINT)](./codebase-maintenance.aps.md)
for the original concrete maintenance intents, and by
[continuous-improvement-backlog (CIB)](../../modules/continuous-improvement-backlog.aps.md)
for new standing improvement intake.

## Why retired

This module never graduated past a bulleted sketch of ten "CI-00X" items.
Every concrete intent (shared utility extraction, Nx generators, CLI helper
consolidation, large module decomposition, DX improvements) is already
tracked under MAINT or its dependents. Keeping a competing meta-module
invited drift and duplicated tracking without adding capacity.

Future improvement work should:

- Add a new CIB-NNN item for focused cross-project intake, or
- Add the item to a specific active module when ownership is clear, or
- Open a new module when a cluster of related improvements justifies it
  (e.g. a dedicated DX theme with concrete deliverables).

Do **not** re-open this retired module as a standing bucket. Use CIB instead,
with executable items and validation metadata.
