---
id: save-time-validation
title: Save-Time Validation
description:
  How daemon-backed save-time validation works — what is watched, assurance
  states, workspace confinement, fallback behaviour, and the ANVIL_WATCH_DAEMON
  routing control.
sidebar_position: 6
---

# Save-Time Validation

| Type        | Authority     | Owner                                                                                                                                          | Status | Freshness                                                     |
| ----------- | ------------- | ---------------------------------------------------------------------------------------------------------------------------------------------- | ------ | ------------------------------------------------------------- |
| Public docs | Authoritative | UJ ([`plans/archive/modules/user-journey.aps.md`](https://github.com/eddacraft/anvil-001/blob/main/plans/archive/modules/user-journey.aps.md)) | Live   | Last reviewed 2026-07-06 for DSV-051 headless driver closeout |

| Upstream                                                                 | Downstream                                                            |
| ------------------------------------------------------------------------ | --------------------------------------------------------------------- |
| `anvil watch`, `anvil start`, the intercept daemon, `ANVIL_WATCH_DAEMON` | Operators and beta testers running daemon-backed save-time validation |

Save-time validation is anvil's daily-driver protection layer: every time you
(or your AI agent) save a file, the change is validated within milliseconds.
This guide is the one place that explains how the pieces fit — the watcher, the
daemon, assurance states, confinement, and what happens when the daemon is not
there. For per-flag reference detail, see the
[configuration reference](../operations/config.md); for the editor/agent
pre-write surface, see the [MCP integration guide](../integrations/mcp.md). For
operator recovery around the detached save-time child, use the
[save-time background driver runbook](../../../runbooks/save-time-background-driver.md).

## What is watched

`anvil watch` observes your repository and runs a code-quality check on each
save (a bare `anvil watch` runs the `check` action by default; `--action none`
restores the architecture/dependency-only watch). The initial scan builds
baseline/readiness state — existing repo contents are not reported as new
save-time violations.

Watch skips local tool state, agent worktrees, generated folders, and common
caches by default, including `.claude`, `.opencode`, `.gemini`, `.serena`,
`.worktrees`, `node_modules`, `target`, `dist`, and cache directories. Scope
further with `--patterns`, `--exclude`, `--file`, `--source`, or `--plans`.

## The daemon's role

The intercept daemon keeps one warm validation model per admitted workspace and
serves save-time verbs over owner-only IPC: a Unix domain socket on macOS and
Linux, a named pipe on Windows. In an interactive terminal `anvil start`
auto-starts it and `anvil watch` offers to start one when none is answering, so
daemon-backed protection is the normal path — see
[Daemon lifecycle](#daemon-lifecycle) below.
`anvil intercept start --foreground` remains the low-level operator and
debugging surface, running the daemon attached to your terminal.

From `v0.8.0-beta`, when a live daemon answers the presence probe, `anvil watch`
routes each save through the daemon instead of spawning a per-save subprocess
scan. The daemon validates the changed-path delta against its warm model, so
watch and the editor/agent MCP `anvil_validate_write` tool converge on the same
verdict path — same inputs, same verdict, faster than a cold subprocess.

The `v0.9.0-beta` headless driver path lets `anvil start` attach that same
save-time validation in the background for registered worktrees. When attached,
`anvil status --json` reports `save_time_driver: "attached"`; when absent or
failed, inspect `anvil intercept status` and the driver runbook above instead of
starting a foreground `anvil watch` by default.

The daemon only ever receives workspace-relative paths for files under an
admitted root; it re-derives file identity from disk and never trusts client
hints for a verdict.

## Daemon lifecycle

As of `v0.8.1-beta`, daemon-backed protection is the normal path rather than an
operator-only foreground ceremony. `anvil start` and `anvil watch` manage the
per-user daemon for you on Linux and macOS:

- **`anvil start`** — in an interactive terminal it auto-starts the daemon and
  reports the result on a `daemon:` line (`started…`, or `reusing…` when one is
  already live). A daemon already running is always reused; concurrent
  invocations never start a second one.
- **`anvil watch`** — when no daemon answers, an interactive run offers to start
  one
  (`No save-time daemon is running. Start one now for daemon-backed validation?`).
  Decline and it falls back to the scoped check; a daemon already running is
  reused without prompting.

Opting out and non-interactive behaviour:

- **`--no-daemon`** — on either command, suppresses only the auto-start (or the
  watch offer). With no daemon already running this leaves you on the scoped
  fallback, but a daemon that is already live is still reused. For watch,
  `ANVIL_WATCH_DAEMON=0` is the stricter opt-out that also disables that reuse.
- **`ANVIL_NO_DAEMON`** — the environment equivalent of `--no-daemon` for
  `anvil start`.
- **`ANVIL_WATCH_DAEMON=0`** — the hard opt-out for watch: no start, no offer,
  and **no reuse** even of a live daemon. The routing values are listed below.
- **Headless, `--json`, CI, hooks, and piped output never start, offer, or
  prompt.** They fall back deterministically to the scoped check, so automation
  never hangs waiting for consent or pollutes a JSON stream.
- **`--verify` is read-only and never starts a daemon.**
- **Windows** uses the named-pipe daemon path and DSV-051's manual driver
  checklist for detached-process verification. The Windows daemon is parser-less
  at this cut-line, so planted-driver verification should use an
  antipattern-family finding and should not require `Certified` coverage.

## Assurance states

The daemon tracks one workspace assurance per admitted root — its honest claim
about how trustworthy the current save-time picture is:

- **`clean`** — the model is warm and the last validation pass found the
  workspace consistent.
- **`stale`** — something invalidated the model (for example a config, boundary,
  or policy edit, or a change needing cross-file resolution); a reason is always
  attached, e.g. `stale{cross-file-resolution-needed}`.
- **`pending`** — a full scan is queued but has not started.
- **`running`** — a full scan is in progress.
- **`bounded`** — a full scan completed, but the workspace exceeded the
  file-count cap after the `.gitignore` filter, so coverage is **bounded**: the
  warm graph is populated but known-incomplete, never reported as a complete
  `clean`. The snapshot carries a `scan_coverage` (scanned vs total files) so
  consumers can surface the bound.
- **`unavailable`** — no daemon verdict is possible. Absence always carries its
  reason: `unavailable{daemon-absent}` means no daemon answered. A missing
  daemon is **never** reported as a stale cached `clean`.

## Workspace confinement

The daemon serves save-time validation only for admitted workspace roots. By
default it runs in **open** mode and adopts each repository on first touch. On
shared or multi-tenant machines, confine it to an explicit allow-list:

```bash
anvil workspace list                      # Current mode and allow entries
anvil workspace mode allowlist            # Only serve admitted roots
anvil workspace allow /path/to/repo      # Admit one root (exact match)
anvil workspace allow /srv/work --prefix # Admit an entire subtree
anvil workspace deny /path/to/repo       # Remove an allow entry
anvil workspace mode open                # Back to first-touch adopt
```

Confinement is operator config the daemon reads live — no restart required. In
`allowlist` mode an empty allow-list still serves each connection's primary
check-in root, so confinement never locks you out of the repository you are
working in. `anvil status` shows `· confined: <N>` next to the save-time line
when the daemon is in allowlist mode.

## Fallback behaviour

When no daemon answers — it was never started, or it died mid-session — watch
does not go quiet and does not over-claim:

- It warns **once per disconnect**, not once per save. The advisory names the
  recovery step: run `anvil start` for daemon-backed validation.
- It falls back to a scoped `check` over exactly the changed paths — never a
  whole-repository `--all` walk.
- It reports assurance `unavailable{daemon-absent}` rather than a misleading
  `clean`.
- On reconnect, watch requests a fresh baseline scan (best-effort), so assurance
  normally returns through `stale` rather than resting on a pre-disconnect
  `clean`.

## Routing control: `ANVIL_WATCH_DAEMON`

One environment variable controls save-time routing:

- **unset** (including an empty `ANVIL_WATCH_DAEMON=`) — the default: route
  through a live daemon when one answers the presence probe; stay on the
  scoped-check path otherwise. This variable governs **routing only** — it does
  not itself start a daemon. The interactive start/offer behaviour is the
  separate [daemon lifecycle](#daemon-lifecycle); `--no-daemon` suppresses that
  without disabling reuse of a daemon that is already live.
- **`0`** (also `false` / `off` / `no`, case-insensitive) — opt out of daemon
  routing entirely: no routing, no reuse, no start, and no offer.
- **`1`** (also `true` / `on` / `yes`) — force daemon routing for diagnostics.
  An absent daemon still falls back to the scoped check with a warning — never a
  hard failure — and reports `unavailable{daemon-absent}`.

Any other value carries no explicit opinion and is treated as unset. Opt out
with an explicit false value, not by blanking the variable.

## Reading the posture

`anvil status` is the home screen for the save-time posture:

- With a live daemon: a `Save-time:` line reporting the assurance state and, in
  allowlist mode, the confinement count — for example
  `Save-time: clean · confined: 3`.
- Under default routing with no daemon running: `Save-time: off`, with a pointer
  to run `anvil start`.
- Under forced routing with no daemon:
  `Save-time: unavailable{daemon-absent} (daemon not running)`.
- Only an explicit `ANVIL_WATCH_DAEMON=0` opt-out hides the line.

`anvil watch --help` carries the same routing story on the CLI itself, and the
watch fallback advisory names `anvil start` — you should never need this page to
recover, only to understand.

## Platform notes

The daemon serves owner-only IPC on every supported OS — Unix domain socket on
macOS and Linux, named pipe on Windows. The MCP `anvil_validate_write`
daemon-status correlation has had Windows parity since `v0.7.1-beta`; the
DSV-051 headless-driver cut-line keeps Windows verification manual because the
detached process and console-window observations need a real Windows session.
Save-time routing is exercised most heavily on macOS and Linux in this beta; see
the [beta testing guide](../beta-testing-guide.md) for current known
limitations.

## Related pages

- [Configuration reference](../operations/config.md) — watch flags and the
  `ANVIL_WATCH_DAEMON` reference entry
- [MCP integration guide](../integrations/mcp.md) — the pre-write
  `anvil_validate_write` surface the daemon also serves
- [Agent harness guide](agent-harness.md) — running agents against the same warm
  verdict path
