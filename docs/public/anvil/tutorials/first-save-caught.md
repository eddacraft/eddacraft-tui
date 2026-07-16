---
id: first-save-caught
title: Your First Save Caught
description:
  Activate daemon-backed save-time protection and watch anvil catch a real
  mistake in your own repo.
sidebar_position: 7
---

# Your First Save Caught

This is the daily-value path end to end: activate protection with `anvil start`,
run the save-time watcher, make a deliberately bad save, and read the finding
anvil raises — all in your own repository.

It tells the same story as the in-terminal tutorial: run `anvil tutorial` and
pick the default **ProtectionLoop** path for the guided version of this walk
inside your terminal. This page is the real-repo edition.

## Prerequisites

- anvil installed and authenticated — steps 1 and 2 of the
  [Quickstart](../quickstart.md)
- A TypeScript, JavaScript, or Rust project (the bad-save example below is
  TypeScript)

## 1. The protection loop in 60 seconds

anvil watches your code for the patterns that turn into incidents — silent
escape hatches, unexplained TODOs, `console.log` slipping into prod. The loop
has three steps: scan the change, surface findings, and let your editor or watch
process react. Findings are deterministic — the same input always produces the
same output. Source, findings, and repository metadata stay on your machine; the
separate [anonymous fleet beacon](../operations/telemetry.md) contains only its
documented aggregate allowlist.

In this tutorial the loop runs against a deliberate mistake you will make
yourself, so you can see each step fire.

## 2. Activate protection: `anvil start`

From the root of your project, run the command for your shell:

macOS / Linux:

```bash
cd path/to/your/repo
anvil start
```

Windows PowerShell:

```powershell
Set-Location C:\path\to\your\repo
anvil start
```

`anvil start` is the activation entrypoint: it runs `anvil init` if needed,
baselines the repo, wires MCP entries for the MCP-capable editors it supports or
strongly detects, keeps the full activation evidence path for Cursor and Claude
Code, and ends in one literal protection state:

- `protecting` — MCP pre-write validation is live
- `ready_restart_required` — config is wired; restart your editor to pick it up
- `watching` — save-time watch fallback active (MCP could not attach)
- `needs_action` — a repair hint is provided
- `unsupported` — the repo's language profile is out of scope for this release
- `error` — see the diagnostic output

Activation does not imply your repo is clean of findings: the first activation
baselines what already exists so that future changes are what gets checked. Any
state from `protecting` down to `watching` is enough for this tutorial — the
save-time path works in all of them.

## 3. Start the daemon and the save-time watcher

Save-time validation is served by the anvil daemon, which runs as its own
process — neither `anvil start` nor `anvil watch` launches it for you. Start it
in a dedicated terminal (foreground is the validated launch mode for the beta):

macOS / Linux:

```bash
anvil intercept start --foreground
```

Windows PowerShell:

```powershell
anvil intercept start --foreground
```

Keep that foreground daemon terminal open. Then, in a second terminal or
PowerShell tab, start the watcher:

macOS / Linux:

```bash
anvil watch --source
```

Windows PowerShell:

```powershell
anvil watch --source
```

The initial pass builds baseline and readiness state — existing repository
contents are not reported as new violations. After that, every save is
validated.

When a live daemon answers, watch routes save-time validation through it by
default — faster verdicts, plus a workspace assurance state you can read at any
time with `anvil status`. Without a daemon, watch silently falls back to a
scoped in-process check, so the rest of this walk still works — you just lose
the assurance state. Use a per-command environment variable when you need to
choose daemon routing:

macOS / Linux:

```bash
ANVIL_WATCH_DAEMON=0 anvil watch --source   # opt out
ANVIL_WATCH_DAEMON=1 anvil watch --source   # force daemon routing
```

Windows PowerShell:

```powershell
$env:ANVIL_WATCH_DAEMON = "0"
anvil watch --source   # opt out
```

To force daemon routing instead, set `$env:ANVIL_WATCH_DAEMON = "1"` before
running `anvil watch --source`. When you stop the watcher, remove the override
with `Remove-Item Env:\ANVIL_WATCH_DAEMON`.

See `anvil watch --help` for the routing details.

## 4. Make a deliberately bad save

Create a scratch file containing two well-known escape hatches — `// @ts-ignore`
(silently disables every type check on the next line) and `: any` (an escape
hatch from the type system). Both are catalogued by anvil as escape-hatch
findings, the kind that compound into bugs nobody can trace:

```typescript title="src/anvil-demo.ts"
// @ts-ignore
export function demo(input: any) {
  return input;
}
```

Save the file. That save is the change anvil scans.

## 5. Read the finding

The watcher validates the save and prints one line per finding plus a trailing
assurance line:

```
  AP-004 (warn) — @ts-ignore suppresses all errors [src/anvil-demo.ts:1]
  AP-003 (warn) — Explicit any type usage [src/anvil-demo.ts:2]
anvil watch: clean (2 finding(s))
```

Reading a finding line: the rule ID (`AP-004`), the severity (`warn` — warnings
report without blocking; `error`-severity findings are blocking), a one-line
summary, and the exact file and line. The trailing line is the workspace
assurance after the verdict — `clean` means the daemon's view of your workspace
is current and trustworthy, not that the repo has no findings. If the daemon
cannot certify a change (for example after a config edit or a rename), assurance
reads `stale{reason}` until the next scan settles it.

Fix the file — or just delete the scratch file — and save again: the next
verdict reports zero findings.

## 6. Check your posture: `anvil status`

At any point, ask anvil where you stand:

macOS / Linux:

```bash
anvil status
```

Windows PowerShell:

```powershell
anvil status
```

Status names the protection state from the same closed vocabulary as
`anvil start`, reports the daemon, and — when daemon routing is active —
includes the save-time assurance line you saw in watch:

```
  Save-time: clean
```

If the daemon is not running, status states the off posture rather than
pretending to a stale `clean`:

```
  Save-time: off (run `anvil start` to enable)
```

To probe the full activation state without writing any config, use the read-only
verifier:

macOS / Linux:

```bash
anvil status --verify
```

Windows PowerShell:

```powershell
anvil status --verify
```

## 7. Clean up and next steps

Remove the scratch file if you have not already:

macOS / Linux:

```bash
rm -f src/anvil-demo.ts
```

Windows PowerShell:

```powershell
Remove-Item .\src\anvil-demo.ts -ErrorAction SilentlyContinue
```

You have now seen the whole loop: a change, a verdict, a posture you can verify.
Where to go from here:

- **Upgrade from save-time to pre-write** — save-time watch is the fallback
  tier. Connect your AI editor over MCP and `anvil_validate_write` refuses bad
  writes before they land. See the
  [MCP integration guide](../integrations/mcp.md).
- [Architecture Boundaries](architecture.md) — define layers and let the same
  loop catch boundary violations.
- [Quickstart](../quickstart.md) — the full two-path tour, including the
  discovery path (`anvil welcome`).
