---
id: dashboard
title: Browse local dashboards
description: Open a read-only terminal dashboard over local anvil state.
owner: DOCSYNC
upstream:
  - crates/anvil-cli/src/commands/dashboard/mod.rs
  - crates/anvil-cli/src/commands/dashboard/architecture.rs
  - crates/anvil-cli/src/commands/dashboard/drift.rs
  - crates/anvil-cli/src/commands/dashboard/suppressions.rs
verified_against: 0.9.0-beta
---

# Browse local dashboards

**For:** users who want an interactive terminal view of retained local evidence

**Time:** 2 minutes

**Outcome:** inspect protection health, gate results, architecture, drift, or
suppression state without changing them

anvil ships **terminal** dashboard surfaces that are **read-only**. They only
read artefacts already written under your project; they never run a scan of
their own and never write.

## Terminal dashboards

Open the terminal picker:

```text
anvil dashboard
```

Or open a named view:

```text
anvil dashboard architecture
anvil dashboard drift
anvil dashboard suppressions
```

For scripts or non-interactive sessions, use the underlying command with
`--json` instead of the interactive surface.

## Next step

Use [weekly insights](insights.md) for a concise activity summary.
