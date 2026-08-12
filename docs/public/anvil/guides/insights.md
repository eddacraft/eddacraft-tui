---
id: insights
title: Review local insights
description:
  Review weekly activity, drift, and suppression health without sending source
  code.
owner: INSIGHTS
upstream:
  - crates/anvil-cli/src/commands/insights.rs
  - crates/anvil-cli/src/insights/mod.rs
  - schemas/anvil-insights.v2.json
verified_against: 0.9.0-beta
---

# Review local insights

**For:** users with retained local anvil activity

**Time:** 5 minutes

**Outcome:** understand recent protection activity and areas needing attention

Run:

```text
anvil insights
```

Focused views include:

```text
anvil insights --drift
anvil insights --suppressions
anvil insights --cumulative
```

Use `--json` for machine-readable data. A shareable scorecard contains headline
counts rather than source code, but inspect any generated file before sharing
it.

No activity can mean that protection has not run, evidence has expired, or there
were simply no matching events. Confirm with `anvil status` rather than
guessing.

Detailed insight rows remain local in the current public beta.

## Next step

Read [evidence and audit trails](../concepts/audit-trail.md).
