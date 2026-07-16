---
id: developer-acceleration
title: Developer Acceleration
description:
  Wire anvil into your AI coding agent — graph context in, pre-write validation
  on its edits, and a fast save-time loop.
sidebar_position: 5
---

# Developer Acceleration

When an AI coding agent writes code, anvil sits in the loop three ways: it gives
the agent graph context so it writes code that fits your project, it validates
the agent's edits before they land, and it runs a fast save-time check so you
catch issues in seconds instead of at CI. This tutorial wires those three up.

It tells the same story as the in-terminal walk: run `anvil tutorial` and pick
the **Developer acceleration** path for the guided version inside your terminal.
This page is the standalone edition.

anvil works with any MCP-capable editor or agent. It ships first-class support
for Cursor and Claude Code today, and any other MCP client can be pointed at
anvil with `anvil mcp-config`. The `anvil` subcommands below are the same on
macOS, Linux, and Windows; only path and environment-variable syntax differs,
and that is called out where it matters.

## Prerequisites

- anvil installed and authenticated — steps 1 and 2 of the
  [Quickstart](../quickstart.md)
- An MCP-capable editor or agent (for example Cursor or Claude Code), or any MCP
  client you can point at a command
- A project in a language anvil supports (TypeScript, JavaScript, or Rust)

## 1. anvil in your AI dev loop

Three things happen when anvil is in the loop with your agent:

- **Graph context in.** anvil exposes your codebase's identity and structure to
  the agent over MCP, so it writes code that respects your symbols and
  boundaries from the start — instead of guessing.
- **Pre-write validation.** With anvil attached over MCP, the agent's edits are
  checked _before_ they are written, so a bad change is caught at the source.
- **A fast save-time loop.** `anvil watch --source` re-checks files as you (or
  the agent) save them and surfaces findings in seconds — your inner loop, not a
  CI round-trip.

Source, findings, and repository metadata stay on your machine, and results are
deterministic: the same input always produces the same output. The separate
[anonymous fleet beacon](../operations/telemetry.md) contains only its
documented aggregate allowlist.

## 2. Wire your agent over MCP

anvil talks to your editor or agent over MCP (the Model Context Protocol).
`anvil start` writes the MCP entry for the MCP-capable editors it detects:

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

For any other MCP client, generate the config explicitly and point your client
at it. `anvil mcp-config` prints the config by default, or writes it to the
target's well-known path with `--write`:

```bash
# Preview the config for a target editor
anvil mcp-config --target cursor
anvil mcp-config --target claude-code

# Write it to the target's well-known path
anvil mcp-config --target cursor --write
```

`--target` names the client whose config format anvil emits; the two shipped
targets are examples, not the whole list of MCP-capable clients you can wire up.

## 3. Confirm pre-write validation

Run the read-only verifier to see whether pre-write validation is live for your
agent's edits. `anvil start --verify` changes nothing — it probes your config,
the MCP client entries of any MCP-capable editor it detects, the activation
baseline, and the repo language profile, then prints one literal protection
state:

```bash
anvil start --verify
```

- `protecting` — MCP pre-write validation is live for the agent's edits
- `ready_restart_required` — config is wired; restart your editor to pick it up
- `watching` — save-time watch fallback active (MCP could not attach)
- `needs_action` — a repair hint is provided
- `unsupported` — the repo's language profile is out of scope for this release
- `error` — see the diagnostic output

Activation does not imply your repo is clean of findings: the first activation
baselines what already exists so that future changes are what gets checked.
Mutating activation is `anvil start` (without `--verify`); the verifier is
always safe to re-run.

## 4. The fast save-time loop

Pre-write validation catches the agent before a write; `anvil watch --source`
closes the loop on everything else. It re-checks source files as they are saved
and reports findings in seconds:

```bash
anvil watch --source
```

The initial pass builds baseline and readiness state — existing repository
contents are not reported as new violations. After that, every save is
validated. When a live daemon answers, watch routes validation through it for
faster verdicts plus a workspace assurance state you can read with
`anvil status`; without one it falls back to a scoped in-process check, so the
loop still works. Choose routing per command with an environment variable when
you need to:

macOS / Linux:

```bash
ANVIL_WATCH_DAEMON=0 anvil watch --source   # opt out of daemon routing
ANVIL_WATCH_DAEMON=1 anvil watch --source   # force daemon routing
```

Windows PowerShell:

```powershell
$env:ANVIL_WATCH_DAEMON=0; anvil watch --source   # opt out
$env:ANVIL_WATCH_DAEMON=1; anvil watch --source   # force daemon routing
```

## 5. Graph context for your agent

The graph context anvil gives the agent is **identity-level by default** — the
names and structure it needs to write conformant code, without shipping your
source text anywhere. Sending source-text snippets is a separate, explicit
opt-in per workspace.

Check the effective state and where it comes from with the read-only status
command:

```bash
anvil gctx egress status
```

If you want to allow source-text snippets for this workspace, opt in explicitly
(and revoke at any time):

```bash
anvil gctx egress enable    # consent to snippet egress for this workspace
anvil gctx egress disable   # revert to identity-only
```

## 6. Wire it up for real

That is the loop: graph context in, pre-write validation on the agent's edits,
and a fast save-time check for everything else. To make it live in your repo:

1. Run `anvil start` (or `anvil mcp-config --target <editor> --write`) to wire
   your MCP-capable editor.
2. Run `anvil watch --source` for the save-time loop.
3. Re-run `anvil start --verify` any time to read the honest state.

---

**Previous:** [Drift Detection](drift.md) | **Next:**
[Your First Save Caught](first-save-caught.md)

**Learn more:**

- [Your First Save Caught](first-save-caught.md) — the daily-value activation
  walk in your own repo
- [Quickstart](../quickstart.md) — install, authenticate, and take a path
- [GitHub integration](../integrations/github.md) — carry checks into CI
