# Local Dashboard

| Type  | Authority     | Owner | Status | Freshness                                                                                             |
| ----- | ------------- | ----- | ------ | ----------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | DASH  | Live   | Last reviewed 2026-08-20 against `dashboard.web` feature flag + `--web` exclusive of `[NAME]` (#4058) |

| Upstream                                                                                                                                                                                                  | Downstream                                               |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| `crates/anvil-dashboard-server/`, `apps/dashboard/`, [ADR-104](../../plans/decisions/104-dashboard-host-server-module-boundary.md), `dashboard.web` in [`flags/manifest.json`](../../flags/manifest.json) | `docs/guides/README.md`, `CHANGELOG.md`, release runbook |

`anvil dashboard --web` opens a read-only browser view of your project's anvil
state — current protection health, gate runs, warnings, and APS plans — served
from your own machine.

## Feature flag (default-off)

The browser surface is gated by the `dashboard.web` rollout flag
(**default-off** for the `v0.10.0-beta` cut). Terminal `anvil dashboard`
surfaces (architecture / drift / suppressions / gate-summary) are **not**
affected. `--web` cannot be combined with a dashboard name — named dashboards
are terminal-only.

Opt in for a session:

```bash
ANVIL_DASHBOARD_WEB=1 anvil dashboard --web
# or a full developer session:
ANVIL_DEV=1 anvil dashboard --web
```

`ANVIL_DASHBOARD_WEB=0` forces the surface off even when `ANVIL_DEV=1` is set.
When the gate is closed, the CLI prints how to opt in and exits non-zero (under
`--json`, a `feature_disabled` envelope on stdout).

## Open it

```bash
ANVIL_DASHBOARD_WEB=1 anvil dashboard --web
```

anvil binds a free loopback port, prints the URL, and opens your browser:

```text
anvil dashboard

  URL        http://127.0.0.1:41293/
  Workspace  /home/you/your-project
  Access     read-only, this machine only

  Opened in your browser.

Press Ctrl-C to stop.
```

Useful flags:

- `--port <PORT>` — pin a port instead of letting the OS choose one. Handy for a
  bookmark; fails with a clear message if the port is taken.
- `--no-open` — print the URL without launching a browser (remote shells, tmux).
- `--json` — emit a one-line startup envelope (`url`, `workspace`, `access`,
  `uiBundled`) before serving, for scripts that need the URL.

Without `--web`, `anvil dashboard` still opens the **terminal** dashboards
(`architecture`, `drift`, `suppressions`, `gate-summary`). The two surfaces are
independent; `--web` cannot be combined with a dashboard name.

## What you will see

The dashboard reads artefacts anvil has already written to your project — it
never runs a scan of its own, and it never writes:

- **Overview** — current protection claim, assurance, save-time posture, and
  what needs attention.
- **Gates** — the latest gate run and its check tree.
- **Warnings** — findings with grouping and filtering, plus the anti-pattern
  reference.
- **Plans** — the APS work-item view.

Most of it is derived from `.anvil/gates.json`, which `anvil gate` writes. **In
a project where you have never run `anvil gate`, the dashboard opens with empty
states rather than invented numbers.** Run one first for a populated view:

```bash
anvil gate
anvil dashboard --web
```

Views state their own data honestly: a panel with no backing artefact says so
and names the gap, instead of rendering a zero that reads like a clean result.

## What it is not

- **Not multi-user and not networked.** The listener refuses any non-loopback
  address, and requests whose `Host` or `Origin` is not the loopback authority
  it bound are rejected. There is no authentication because nothing is reachable
  from another machine — do not port-forward or reverse-proxy it.
- **Not a control surface.** Every endpoint is read-only; the dashboard cannot
  approve, suppress, or change anything.
- **Not a team view.** It shows the one workspace you launched it in. A
  cross-repository team surface is separate, later work.

## When the UI is not bundled

The dashboard UI is compiled into the `anvil` binary. Released binaries always
carry it — the release build fails rather than shipping without it.

A binary you built yourself from a checkout will not have it unless you built
the app first. In that case the URL serves the read-only API and says so
plainly. To bundle it:

```bash
pnpm --filter @eddacraft/anvil-dashboard build
cargo build -p eddacraft-anvil
```

`ANVIL_DASHBOARD_DIST` points the build at a `dist` directory elsewhere;
`ANVIL_DASHBOARD_REQUIRE_BUNDLE=1` turns a missing bundle into a build failure
(the release pipeline sets both).

## Developing the dashboard

For UI work, run the Vite dev server against a live API for hot reload:

```bash
anvil dashboard --web --port 4217 --no-open   # terminal 1: the API
pnpm --filter @eddacraft/anvil-dashboard dev  # terminal 2: the UI on :5174
```

The dev server proxies `/api` and `/openapi.json` to port 4217, so the browser
only ever talks to one origin and the loopback guard stays satisfied. That proxy
is already configured in `apps/dashboard/vite.config.ts`; pointing the browser
straight at 4217 for a hot-reload session is not a supported setup.
