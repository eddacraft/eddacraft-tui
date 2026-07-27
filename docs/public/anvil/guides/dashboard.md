---
id: dashboard
title: Browse local dashboards
description:
  Open a read-only browser or terminal dashboard over local anvil state.
---

# Browse local dashboards

**For:** users who want an interactive view of retained local evidence

**Time:** 2 minutes

**Outcome:** inspect protection health, gate results, architecture, drift, or
suppression state without changing them

anvil ships dashboard surfaces that are **read-only**. They only read artefacts
already written under your project; they never run a scan of their own and never
write.

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

## Browser dashboard

Newer betas after 0.9.0-beta add a loopback **browser** dashboard for protection
health, gate runs, warnings, and plans, bundled inside the `anvil` binary (no
separate download or Node toolchain). Discover it from the installed binary:

```text
anvil dashboard --help
```

When your binary lists a web/browser mode (commonly `--web`):

- anvil binds a free loopback port, prints the URL, and can open your browser;
- a port flag can pin a bookmark; a no-open flag prints the URL only;
- panels open with honest empty states if you have not yet run `anvil gate`;
- the listener accepts only loopback traffic — do not port-forward or
  reverse-proxy it.

```text
anvil gate
anvil dashboard --help
```

The installed help is authoritative for flag names on your version. Browser and
terminal surfaces are independent; do not combine a web mode with a named
terminal dashboard.

## Next step

Use [weekly insights](insights.md) for a concise activity summary.
