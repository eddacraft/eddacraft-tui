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
  - schemas/anvil-insights.v3.json
verified_against: 0.9.0-beta
---

# Review local insights

**For:** users with retained local anvil activity

**Time:** 5 minutes

**Outcome:** understand recent protection activity and areas needing attention

Insights are **local-only**. They summarise activity on this machine. They are
not a team dashboard, and they are not uploaded with the anonymous usage beacon.

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

Several weekly metrics are not instrumented yet. The default JSON document
(`anvil.insights.v1`) reports them as `0`, which reads the same as a measured
zero. Add `--schema v3` to get `anvil.insights.v3`, which reports them as `null`
instead, so a consumer can tell an unmeasured metric from a real one. Today only
`witness_events_observed` is measured. The human-readable output says "not yet
measured" under either schema.

No activity can mean that protection has not run, evidence has expired, or there
were simply no matching events. Confirm with `anvil status` rather than
guessing.

Detailed insight rows remain local in the current public beta.

## Next step

Read [evidence and audit trails](../concepts/audit-trail.md).

## Related definitions

- [How anvil evaluates a project](../concepts/evaluation-model.md)
- [Evidence and audit trails](../concepts/audit-trail.md)
- [Anonymous usage telemetry](../operations/telemetry.md)
