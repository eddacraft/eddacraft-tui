---
id: validation
title: CLI Reference
description: The aps CLI — authoring, orchestration, audit, and CI integration.
sidebar_position: 1
---

# CLI Reference

| Type        | Authority | Owner   | Status | Freshness                                               |
| ----------- | --------- | ------- | ------ | ------------------------------------------------------- |
| Public docs | Derived   | DOCSYNC | Live   | Last reviewed 2026-06-22 against anvil-plan-spec v0.4.0 |

| Upstream                                                                  | Downstream            |
| ------------------------------------------------------------------------- | --------------------- |
| [anvil-plan-spec](https://github.com/EddaCraft/anvil-plan-spec) `docs/**` | APS docs-site section |

The `aps` CLI has two layers:

- **Authoring** — scaffold projects, lint specs (`init`, `update`, `migrate`,
  `lint`)
- **Orchestration** — drive work items through the lifecycle (`next`, `start`,
  `complete`, `graph`)

You can ignore orchestration and edit markdown by hand — the CLI is additive.

## Command index

```bash
aps init [dir]              # Create APS structure
aps update [dir]            # Refresh templates and tool files
aps migrate [dir]           # Convert v1 layout to v2 (.aps/)
aps lint [file|dir]         # Validate APS documents
aps next [module]           # Show next ready work item
aps start <ID>              # Mark Ready → In Progress
aps complete <ID>           # Mark In Progress → Complete
aps graph [module]          # Dependency graph
aps audit [module]          # Audit plan state against reality
aps doctor                  # Diagnose global binary vs vendored CLI
aps setup [component]       # Add integrations (hooks, agents, tools)
aps upgrade [--apply]       # Remove generated bloat
aps --help                  # Top-level help
```

Every command accepts `--plans <dir>` if plans are not at the default `plans/`
location.

## Project config discovery

Project-scoped commands resolve their plan root automatically. `aps` walks up
from the current directory for the nearest `.aps/config.yml` and uses its
`plans_dir`.

Resolution order: explicit `--plans` / target → `APS_PLANS` env var → discovered
`plans_dir` → `plans/`.

Add `--strict` (or `APS_STRICT=1`) to fail on toolchain version drift:

```bash
aps lint --strict
```

## `aps lint`

```bash
aps lint                          # Lint plans/
aps lint plans/modules/auth.aps.md
aps lint . --json                 # Machine-readable output
```

See [Validation rules →](../spec/determinism.md) for error and warning codes.

## Orchestration

The orchestration commands read and rewrite `.aps.md` files in place. Markdown
stays the single source of truth.

### State machine

```text
Draft ──→ Ready ──→ In Progress ──→ Complete
```

| Command        | Transition                                          |
| -------------- | --------------------------------------------------- |
| `aps next`     | Read-only                                           |
| `aps start`    | Ready → In Progress (dependencies must be Complete) |
| `aps complete` | In Progress → Complete                              |
| `aps graph`    | Read-only                                           |
| `aps audit`    | Read-only (executes Validation commands by default) |

### `aps next`

```bash
$ aps next
AUTH-003: Implement token refresh
Module: AUTH | Dependencies: AUTH-001, AUTH-002 | Status: Ready
File: plans/modules/auth.aps.md

$ aps next auth          # Scope to one module
```

### `aps start <ID>`

```bash
$ aps start AUTH-003
Marked AUTH-003 as In Progress
Suggested branch: work/auth-003
Context package: .aps/context/AUTH-003.md
```

On success:

- Rewrites `- **Status:**` to `In Progress`
- Suggests a branch name (`work/<id>`)
- Writes a context package at `.aps/context/<ID>.md`

### `aps complete <ID>`

```bash
$ aps complete AUTH-003 --learning "Token refresh needs retry on network errors"
Marked AUTH-003 as Complete: 2026-05-12
Learning recorded for AUTH-003
```

### `aps graph [module]`

```bash
$ aps graph auth
AUTH-001 [Complete] Create users
  <- none
AUTH-002 [Complete] Verify credentials
  <- AUTH-001[Complete]
AUTH-003 [Ready] Add token refresh
  <- AUTH-001[Complete] AUTH-002[Complete]
```

### `aps audit [module]`

```bash
$ aps audit
Complete-item verification:
  AUTH-001     PASS     npm test -- auth.test.ts
  AUTH-002     FAIL     npm test -- session.test.ts

Findings:
  A001  AUTH-002     overstated: Validation failed
  A003  UI-002       stale: module last reviewed 89 days ago

Findings: 2 (23 items audited)
```

Options: `--json`, `--no-run` (skip executing validation commands),
`--stale-days N`.

> **CI safety:** Use `aps audit --no-run` in pull-request jobs.

### `aps doctor`

Read-only diagnostics for global binary vs vendored CLI state:

```bash
$ aps doctor
  [ok  ] global binary: aps 0.4.0 at ~/.aps/bin/aps
  [warn] cli_version: project pins 0.3.0 but this binary is 0.4.0
  [warn] vendored CLI: leftover bin/aps, lib/ — run `aps upgrade`
```

## End-to-end loop

```bash
aps next
aps start AUTH-003
git switch -c work/auth-003
# ...implement, test, commit...
aps complete AUTH-003 --learning "..."
aps next
```

## MCP server

An optional MCP server in
[anvil-plan-spec `mcp/`](https://github.com/EddaCraft/anvil-plan-spec/tree/main/mcp)
exposes orchestration commands as a single `aps` tool over stdio.

```json
{
  "mcpServers": {
    "aps": {
      "command": "node",
      "args": ["/path/to/anvil-plan-spec/mcp/src/index.ts"],
      "env": { "APS_PLANS": "/path/to/your/project/plans" }
    }
  }
}
```

## CI integration

```yaml
name: Lint APS Documents

on:
  push:
    paths: ['plans/**/*.aps.md', 'plans/**/*.actions.md']
  pull_request:
    paths: ['plans/**/*.aps.md', 'plans/**/*.actions.md']

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install APS CLI
        run: |
          curl -fsSL https://raw.githubusercontent.com/EddaCraft/anvil-plan-spec/main/scaffold/install \
            | bash -s -- --global
      - name: Lint APS documents
        run: aps lint plans/ --strict
```

---

**Back to:** [APS Overview →](../overview.md)
