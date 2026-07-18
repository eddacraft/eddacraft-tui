---
id: dashboard
title: Browse local dashboards
description: Open a read-only terminal dashboard over local anvil state.
---

# Browse local dashboards

**For:** users who want an interactive view of retained local evidence

**Time:** 2 minutes

**Outcome:** inspect architecture, drift, or suppression state without changing
it

Run the picker:

```text
anvil dashboard
```

Or open a named view:

```text
anvil dashboard architecture
anvil dashboard drift
anvil dashboard suppressions
```

The dashboard is a terminal user interface and reads local state. For scripts or
non-interactive sessions, use the underlying command with `--json` instead.

If your installed version does not recognise a view, run
`anvil dashboard --help`; the installed binary is authoritative.

## Next step

Use [weekly insights](insights.md) for a concise activity summary.
