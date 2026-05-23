---
id: quickstart
title: Quickstart
description: Scan your project and see what anvil finds in under 5 minutes.
sidebar_position: 3
---

# Quickstart

Install anvil, scan your project, and fix your first issue -- all in under 5
minutes.

## Prerequisites

- A TypeScript or JavaScript project
- A terminal (macOS, Linux, or Windows)

## Install

:::info Beta

anvil is currently in beta — the latest tagged release is `v0.7.1-beta`. If your
team has gated beta access, use the GitHub account tied to that access when
prompted by anvil or the docs site. See the
[beta testing guide](/anvil/beta-testing-guide) for the current scope and known
gaps, and the v0.6.x → v0.7.0-beta migration note if you're upgrading an
existing `0.6.x` install. See the
[archived migration note](https://github.com/eddacraft/anvil-001/blob/main/docs/archive/runbooks/v0.6.x-to-v0.7.0-beta-migration.md)
for the operator delta.

:::

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh

# Windows (PowerShell)
irm https://install.eddacraft.ai/windows | iex

# Or via Homebrew (macOS / Linux)
brew install eddacraft/tap/anvil

# Or via WinGet (Windows)
winget install eddacraft.anvil

# Or via Scoop (Windows)
scoop bucket add eddacraft https://github.com/eddacraft/scoop-bucket
scoop install anvil
```

anvil is a single native binary available for macOS, Linux, and Windows. Your
project still needs Node.js and a package manager for lint and test gate checks,
but anvil itself has no runtime dependencies.

If Anvil is already installed through Homebrew, the macOS/Linux curl installer
will not replace that install. It exits successfully and tells you to run:

```bash
brew upgrade eddacraft/tap/anvil
```

Use one install method per machine. To move from Homebrew to the standalone
installer, uninstall the Homebrew formula first.

:::tip Windows users

If the installer doesn't add `anvil` to your PATH automatically, add
`%USERPROFILE%\.eddacraft\bin` to your system PATH or run:

```powershell
$env:Path = "$env:USERPROFILE\.eddacraft\bin;$env:Path"
```

:::

## Authenticate

Start the default device-code login flow:

```bash
anvil auth login
```

anvil prints a short code and verification URL. Open the URL in your browser,
enter the code, and the CLI will finish the login automatically.

If you need email OTP instead, run:

```bash
anvil auth login --otp
```

## Activate (`anvil start`)

From the root of your project, run:

```bash
cd path/to/your/repo
anvil start
```

`anvil start` is the activation entrypoint. It runs `anvil init` if needed,
baselines the repo, wires Cursor and Claude Code MCP entries (writing
`~/.cursor/mcp.json` and `~/.claude.json`), and ends in one literal protection
state — one of:

- `protecting` — MCP pre-write validation is live
- `ready_restart_required` — config is wired, restart Cursor/Claude Code to pick
  it up, or follow the daemon repair hint if the daemon is unreachable,
  unenforced, stale, or all-quarantined
- `watching` — save-time watch fallback active (MCP could not attach)
- `needs_action` — repair hint provided
- `unsupported` — repo language profile is out of scope (e.g. Python or Rust in
  this release)
- `error` — see the diagnostic output

When the daemon is running and reachable over owner-only IPC, the
`anvil_validate_write` MCP tool runs through the daemon-backed path; an embedded
scanner is the correctness-equivalent fallback when the daemon is not available.
In `v0.7.x-beta` the protection-claim contract is defined across
`anvil status --json`, `anvil doctor --json`, the `anvil_validate_write`
MCP-tool response, and the TypeScript driver-client. The MCP response only
carries `protection_claim` when the daemon is reachable. As of `v0.7.1-beta`,
Windows reaches the daemon through the named-pipe client too, so
`correlation.daemonStatus` and `protection_claim` are no longer Unix-only.

To probe state without writing config:

```bash
anvil start --verify
```

To run activation and then enter the save-time watch fallback in the same
process:

```bash
anvil start --watch
```

Watch mode is **save-time fallback only** — never claimed equivalent to MCP
pre-write interception.

The initial watch pass builds baseline/readiness state and does not report the
existing repository as new violations. Watch reports findings after later file
changes. It also prints startup feedback immediately and falls back to plain
output when stdin or stdout is not a terminal.

## Scan Your Project

You can also surface findings directly with the targeted source-analysis
command:

```bash
anvil check --all
```

`anvil check` is the targeted source-analysis command. In the current Rust CLI
it scans source artefacts for Anvil's registry-backed anti-pattern rules,
including architecture-category findings emitted by that scanner. Use
`anvil gate` when you want the full workflow judgement across architecture,
policy, secrets, and other gate checks.

Most projects have something. Here is typical `check` output:

```
Checked 12 file(s)

Warnings
----------------------------------------
  ⚠ [AP-003] Explicit any type detected
  src/utils/parser.ts:42
  Using 'any' defeats type safety

  ⚠ [AP-006] Empty catch block
  src/services/auth.ts:87
  Empty catch blocks hide errors

Summary
----------------------------------------
  Total            2
  Warnings         2
  Time             42ms
```

If everything passes, you will see:

```
Checked 12 file(s)

  ✓ No warnings found
```

To run the broader gate surface, use:

```bash
anvil gate --profile dev
```

## Diagnostics

Verify state and the binary you're running:

```bash
anvil status --verify     # Read-only activation probe (same backend as `anvil start --verify`)
anvil version             # Current and latest version + the upgrade command for your install method
anvil doctor              # Environment health check
```

`anvil version` is install-method aware — it knows whether you used Homebrew,
Scoop, WinGet, the install script, or a developer build, and prints the right
upgrade command for that path.

## Turn On Watch Mode (fallback)

If `anvil start` finished in `watching` rather than `protecting`, anvil is
already running the save-time fallback. To run a standalone watcher in another
terminal:

```bash
anvil watch --source
```

Watch mode is the save-time fallback for the AI guardrail — useful when MCP
pre-write attach is not available, but never equivalent to pre-write
interception.

```
Anvil Watch

Watching for changes...
Press Ctrl+C to stop.
```

Save a file and see anvil catch it. Every change is validated in milliseconds,
not minutes.

Audit and watch skip local tool state, agent worktrees, generated folders, and
common caches by default, including `.claude`, `.opencode`, `.gemini`,
`.serena`, `.worktrees`, `node_modules`, `target`, `dist`, and cache
directories.

:::tip

Run watch mode in a dedicated terminal pane or use the VS Code extension for
in-editor diagnostics.

:::

## Fix Your First Issue

Take one of the warnings from the scan -- say AP-003, the explicit `any` in
`src/utils/parser.ts`:

**Before:**

```typescript
export function parse(input: any): Record<string, unknown> {
  // ...
}
```

**After:**

```typescript
export function parse(input: string): Record<string, unknown> {
  // ...
}
```

Save the file. If watch mode is running you will see immediate confirmation:

```
Change detected: src/utils/parser.ts

Checking anti-patterns... done

All gates passed.
```

One warning down. Repeat for the rest at your own pace.

## Next Steps

- **Interactive tutorial** -- run `anvil tutorial` for a guided walk-through
  inside your terminal
- [Set up your first project](/anvil/first-project) -- architecture boundaries,
  suppressions, and CI
- [Understand gates](/anvil/concepts/gates) -- what anvil validates and why
- [Configuration reference](/anvil/operations/config) -- customise checks,
  patterns, and watch behaviour
- [Uninstalling Anvil](./operations/uninstall.md) -- clean project or
  machine-level reset after beta testing

**Feature tutorials:**

- [Custom policies](/anvil/tutorials/policies) -- write OPA/Rego rules for your
  team's standards
- [Architecture boundaries](/anvil/tutorials/architecture) -- define and enforce
  module boundaries
- [Drift detection](/anvil/tutorials/drift) -- capture snapshots and track
  architectural drift
- [CI integration](/anvil/tutorials/ci) -- add anvil to your pipeline

**v0.7.x-beta operator surfaces** (runbooks are the source of truth):

- [`anvil-run` — wrapped-launch ingress](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-run.md)
  — route `claude`, `codex`, `aider`, and similar agents through a
  daemon-attributed session. Stable exit codes (`64 / 69 / 73 / 75 / 78`) and
  shell-integration script.
- [`anvil l4-validate`](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — dedicated L4-policy validator for CI / pre-push consumers; replaces the
  prior `anvil hook pre-push` reuse.
- Protection-claim contract — `anvil status --json | jq '.claim.worktree_state'`
  and the same `ProtectionClaim` shape on `anvil doctor --json`, the
  `anvil_validate_write` MCP-tool response when the daemon is reachable, and the
  TypeScript driver-client. Windows MCP protection-claim parity landed in
  `v0.7.1-beta`.
- `anvil baseline --new-identity` / `anvil start --new-identity` — fork opt-out:
  mint a fresh `project_uuid` and record the previous one as `forked_from`.
- [`anvil intercept unblock --acknowledge-cascade`](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/v0.7.0-beta-release-runbook.md)
  — clear a `degraded:fence-cascade` rate-limited fence. The `RateWindow`
  capacity is 4, so the **fifth** fire within a 60 s window engages cascade; use
  this flag once the incident is resolved.
- [Witness chain](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-witness-chain.md)
  and
  [hook coexistence](https://github.com/eddacraft/anvil-001/blob/main/docs/runbooks/anvil-hook-coexistence.md)
  — what `anvil/witness/` writes into your repo and how Anvil registers under
  lefthook / husky / pre-commit-framework instead of overwriting `.git/hooks/`.

---

**Next:** [Set up your first project →](/anvil/first-project)

**Need help?** Check [Troubleshooting](/anvil/operations/troubleshooting) or
[open an issue](https://github.com/eddacraft/anvil/issues).
