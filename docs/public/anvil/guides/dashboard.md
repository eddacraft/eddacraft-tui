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

anvil ships two independent dashboard surfaces. Both are **read-only** and read
only artefacts already written under your project.

## Browser dashboard

Open a loopback browser view of protection state, gate runs, warnings, and
plans:

```text
anvil dashboard --web
```

anvil binds a free loopback port, prints the URL, and opens your browser. Useful
flags:

```text
anvil dashboard --web --port 41293
anvil dashboard --web --no-open
```

- `--port` pins a port for a bookmark (fails clearly if the port is taken).
- `--no-open` prints the URL without launching a browser (remote shells, tmux).

The browser surface is bundled inside the `anvil` binary — no separate download
or Node toolchain. It never runs a scan of its own and never writes. In a
project where you have not yet run `anvil gate`, panels open with honest empty
states rather than invented numbers:

```text
anvil gate
anvil dashboard --web
```

The listener accepts only loopback traffic. Do not port-forward or reverse-proxy
it; there is no authentication because nothing is reachable from another
machine.

## Terminal dashboards

Without `--web`, `anvil dashboard` opens the terminal picker:

```text
anvil dashboard
```

Or open a named view:

```text
anvil dashboard architecture
anvil dashboard drift
anvil dashboard suppressions
```

`--web` cannot be combined with a dashboard name. For scripts or non-interactive
sessions, use the underlying command with `--json` instead of either surface.

If your installed version does not recognise a view or flag, run
`anvil dashboard --help`; the installed binary is authoritative.

## Next step

Use [weekly insights](insights.md) for a concise activity summary.
