---
id: config
title: Configuration
description: Complete reference for anvil configuration options.
sidebar_position: 1
---

# Configuration

anvil uses project files plus CLI flags for runtime options. The most important
file today is `.anvilrc`; the architecture file powers import-boundary checks;
`gate-config.json` is a planning surface that is not yet consumed by
`anvil gate`.

## Configuration Files

| File                       | Current status                                                 |
| -------------------------- | -------------------------------------------------------------- |
| `.anvilrc`                 | Active project settings read by `anvil gate`                   |
| `.anvil/architecture.yaml` | Active layer and boundary definitions                          |
| `.anvil/gate-config.json`  | Forward-looking gate composition record, not active gate input |

## Schema Migration

anvil records the version that created a project's config (`created_by_version`)
and can reconcile that config forward as the schema evolves across anvil
versions.

```bash
# Preview the changes for the current version delta (default: dry run)
anvil migrate schema

# Apply the migrated config to disk
anvil migrate schema --apply
```

`anvil migrate schema` is a **dry run by default** — it prints what would change
and writes nothing. Pass `--apply` to write the migrated config. It only applies
migrations registered for the version delta between `created_by_version` and the
running binary, so running it on an already-current project is a no-op.

> This is distinct from `anvil migrate format`, which converts a legacy
> `.anvilrc` to the multi-format `.anvil.<ext>` surface (a filename/encoding
> change, not a schema change). A bare `anvil migrate` still routes to `format`
> for back-compat with a deprecation notice.

## `.anvilrc`

Created by `anvil init`. Supports JSON, YAML, and TOML formats.

`.anvilrc` selects the checks Anvil runs by default when it scans your project.
Those checks produce findings. `anvil gate` then combines those findings with
broader build-and-CI checks to decide whether the workflow gate passes.

### YAML (default)

`anvil init` generates a YAML `.anvilrc` by default.

```yaml
schemaVersion: '1.0.0'
planningDir: plans
format: yaml
checks:
  - secret-detection
  - import-boundaries
  - antipattern-scan
```

### JSON

```json
{
  "schemaVersion": "1.0.0",
  "planningDir": "plans",
  "format": "yaml",
  "checks": ["secret-detection", "import-boundaries", "antipattern-scan"]
}
```

### TOML

```toml
schema_version = "1.0.0"
planning_dir = "plans"
format = "yaml"
checks = ["secret-detection", "import-boundaries", "antipattern-scan"]
```

:::note

JSON and YAML use **camelCase** keys. TOML uses **snake_case** keys.

:::

| Field           | Type     | Default                                                         | Description                         |
| --------------- | -------- | --------------------------------------------------------------- | ----------------------------------- |
| `schemaVersion` | string   | `"1.0.0"`                                                       | Config schema version               |
| `planningDir`   | string   | `"plans"`                                                       | Directory for APS plan files        |
| `format`        | string   | `"yaml"`                                                        | Plan format: `json`, `yaml`, `toml` |
| `checks`        | string[] | `["secret-detection", "import-boundaries", "antipattern-scan"]` | Enabled project checks              |

### Available Checks

| Check               | Description                           |
| ------------------- | ------------------------------------- |
| `secret-detection`  | Detect leaked secrets and credentials |
| `import-boundaries` | Enforce module import boundaries      |
| `antipattern-scan`  | Detect common code anti-patterns      |
| `policy`            | Evaluate OPA policy rules             |
| `command-safety`    | Detect dangerous shell commands       |
| `sql-migrations`    | Detect risky SQL migration patterns   |
| `github-actions`    | Detect risky workflow patterns        |
| `dockerfile`        | Detect Dockerfile build-hygiene risks |
| `shell-scripts`     | Detect shell-script governance risks  |

The four infrastructure-hygiene surfaces are default-on in the current release
window and can be forced for a session with `ANVIL_TRACK_SURFACE_SQL=1`,
`ANVIL_TRACK_SURFACE_GHA=1`, `ANVIL_TRACK_SURFACE_DOCK=1`, or
`ANVIL_TRACK_SURFACE_SH=1`. Set the matching variable to `0` to opt that surface
out for the session.

## Gate Configuration

Managed by `anvil gate-config`. Stored at `.anvil/gate-config.json`.

:::caution Current limitation

`gate-config.json` is visible in the CLI, but it is not the file to edit when
you want to change what `anvil gate` runs today. Use `.anvilrc#checks` for the
project default, or pass `--only-checks` / `--skip-checks` for one run.

:::

Use `anvil gate-config --list` to view the current configuration, and
`--enable <check>` / `--disable <check>` to toggle individual checks.

This file records the intended gate composition — which build-and-CI checks
(`lint`, `test`, `coverage`, `dependency`) and Anvil analysis checks
(`secret-detection`, `import-boundaries`, `antipattern-scan`, `policy`,
`command-safety`) belong to the gate, plus the scoring threshold.

:::note

`anvil gate` does not currently read `.anvil/gate-config.json`. The gate run is
controlled by the `--only-checks` / `--skip-checks` flags and, as a default
filter, the `checks` list in `.anvilrc`. Use `gate-config` to plan and document
the intended gate composition today; wiring it into the `anvil gate` runner is
tracked as follow-up work.

:::

`.anvilrc` sets your project's default analysis checks that `anvil gate`
actually consumes. `gate-config` is the forward-looking surface for the broader
gate run.

When `.anvilrc#checks` contains an unknown check name, `anvil gate` warns with a
did-you-mean suggestion and runs the known subset. This is intentionally more
permissive than `--only-checks` / `--skip-checks`, which fail fast on unknown
names because they apply to one explicit invocation. If no configured checks are
recognised, `anvil gate` fails rather than silently running nothing.

:::note

For the shared Anvil analysis checks, `gate-config` uses the same canonical
names shown in init and `.anvilrc`. Use `secret-detection` and
`import-boundaries`, not older internal names. Legacy aliases like `secret` and
`architecture` are accepted for compatibility, but Anvil normalises them to the
canonical names above.

:::

```json
{
  "version": 1,
  "checks": [
    {
      "name": "lint",
      "description": "Code quality and style checks",
      "enabled": true
    },
    {
      "name": "test",
      "description": "Test suite execution",
      "enabled": true
    },
    {
      "name": "coverage",
      "description": "Code coverage thresholds",
      "enabled": false
    },
    {
      "name": "dependency",
      "description": "Dependency vulnerability scanning",
      "enabled": true
    },
    {
      "name": "secret-detection",
      "description": "Detect leaked secrets and credentials",
      "enabled": true
    },
    {
      "name": "import-boundaries",
      "description": "Enforce module import boundaries",
      "enabled": true
    },
    {
      "name": "antipattern-scan",
      "description": "Detect common code antipatterns",
      "enabled": true
    },
    {
      "name": "policy",
      "description": "Evaluate OPA policy rules",
      "enabled": true
    },
    {
      "name": "command-safety",
      "description": "Detect dangerous shell commands in plan-described scripts",
      "enabled": true
    }
  ],
  "thresholds": {
    "overall_score": 80
  }
}
```

Each check can have an optional `config` object for check-specific settings.
Those settings affect how the check produces findings before the gate evaluates
the overall result.

## Architecture Definition

Architecture boundaries are defined in `.anvil/architecture.yaml`, not in
`.anvilrc`. See the [Architecture tutorial](/anvil/tutorials/architecture) for a
full walkthrough.

Layers are a **map** keyed by layer name. Each layer has `patterns` (glob list)
and `depends_on` (allowed dependencies):

```yaml
schema_version: '0.1.0'
template: custom
layers:
  api-layer:
    patterns:
      - 'src/api/**'
    depends_on:
      - service-layer
      - utils

  service-layer:
    patterns:
      - 'src/services/**'
    depends_on:
      - repository-layer
      - utils

  repository-layer:
    patterns:
      - 'src/repositories/**'
    depends_on:
      - utils

  utils:
    patterns:
      - 'src/utils/**'
    depends_on: []
```

:::caution

The `schema_version` field must be exactly `"0.1.0"`. anvil validates this on
every run and rejects definitions with a different version.

:::

### Templates

Use `template` to start from a preset layer structure. anvil fills in default
patterns and dependencies that you can then customise.

| Template       | Layers                                                     |
| -------------- | ---------------------------------------------------------- |
| `starter`      | components, lib, services                                  |
| `layered`      | presentation, business, data, shared                       |
| `hexagonal`    | core, ports, adapters, application                         |
| `clean`        | entities, use_cases, interface_adapters, frameworks        |
| `ddd`          | domain, application, infrastructure, interfaces            |
| `monorepo`     | packages, shared                                           |
| `serverless`   | functions, services, shared                                |
| `nx-workspace` | apps, feature-libs, data-access-libs, ui-libs, shared-libs |
| `custom`       | (empty — define your own)                                  |

### Validation Options

```yaml
options:
  detect_orphans: true
  detect_circular: true
  default_severity: error
  exclude_patterns:
    - '**/*.test.ts'
    - '**/*.spec.ts'
    - '**/__tests__/**'
    - '**/__fixtures__/**'
    - '**/node_modules/**'
```

Validate basic structure and layer references with `anvil architecture validate`
and inspect the parsed file with `anvil architecture show`. Boundary enforcement
happens when the import-boundaries gate runs.

## Anti-Patterns

Anti-pattern detection is configured per-pattern. There are 18 built-in patterns
grouped into five families: **guardrail-suppression** (AP-001, AP-002, AP-004,
AP-005, GS-001), **type-system-evasion** (AP-003), **error-visibility** (AP-006,
AP-007), **responsibility-laundering** (RL-001..RL-006), and **deferred-debt**
(DD-001..DD-004). 15 are enabled by default; 3 are opt-in. Rules are sourced
from the compiled `.anvil` registry at `patterns/compiled/registry.json`.

### Default Patterns (always active)

| Pattern  | Family                    | Description                              | Severity |
| -------- | ------------------------- | ---------------------------------------- | -------- |
| `AP-001` | guardrail-suppression     | Broad `eslint-disable` added             | warning  |
| `AP-003` | type-system-evasion       | Explicit `any` type usage                | warning  |
| `AP-004` | guardrail-suppression     | `@ts-ignore` suppresses all errors       | warning  |
| `AP-006` | error-visibility          | Empty catch block swallows errors        | warning  |
| `GS-001` | guardrail-suppression     | Non-null assertion overrides nullability | warning  |
| `RL-001` | responsibility-laundering | Unverified "pre-existing" claim          | warning  |
| `RL-002` | responsibility-laundering | Phantom follow-up tracking               | warning  |
| `RL-003` | responsibility-laundering | Blanket unrelated dismissal              | error    |
| `RL-004` | responsibility-laundering | Unverified "not touched" claim           | warning  |
| `RL-005` | responsibility-laundering | Deferred without artifact                | warning  |
| `RL-006` | responsibility-laundering | Reply disguised as fix                   | info     |
| `DD-001` | deferred-debt             | Untracked deferred-work marker           | warning  |
| `DD-002` | deferred-debt             | Untracked shortcut marker                | warning  |
| `DD-003` | deferred-debt             | Temporary code without expiry            | info     |
| `DD-004` | deferred-debt             | Completion claim with outstanding debt   | warning  |

### Opt-in Patterns

Enable with `anvil check --include-opt-in`:

| Pattern  | Family                | Description                     | Severity |
| -------- | --------------------- | ------------------------------- | -------- |
| `AP-002` | guardrail-suppression | Rule-specific `eslint-disable`  | info     |
| `AP-005` | guardrail-suppression | `@ts-expect-error` used         | info     |
| `AP-007` | error-visibility      | Console statement in production | info     |

## Secret Detection

Built-in patterns match common secret formats:

```
api[_-]?key, secret[_-]?key, password, token,
credential, private[_-]?key, bearer, auth
```

High-entropy strings (Shannon entropy > 4.5 bits/character) are also flagged.

## Suppressions

Suppressions are managed via inline comments in your source files.

### Inline

```typescript
// @anvil-ignore AP-003 -- Legacy parser uses any, migration planned Q2
export function parse(input: any): Record<string, unknown> { ... }
```

:::caution

Suppressions without a reason trigger their own warning.

:::

### Tracked exception store (foundation)

Inline `@anvil-ignore` comments are the supported way to suppress findings
today. Alongside them, anvil now has a tracked, project-level exception store at
`anvil/exceptions/store.json`. An older `.anvil/exceptions.json`, if present, is
still honoured as a read-only fallback. A non-destructive migration of the
legacy file to the tracked path is defined (ADR-073) but is not yet surfaced as
an operator command in this release.

:::caution Enforcement not yet wired

The exception store is a foundation only in this release. Hand-written entries
in `anvil/exceptions/store.json` (or the legacy `.anvil/exceptions.json`) **do
not yet suppress findings**, and there is no operator CLI for managing them yet
— use inline `@anvil-ignore` comments for suppression. Enforcement and the
management commands are tracked for a follow-up.

:::

## Watch Mode

Watch mode is configured via CLI flags, not config files.

```bash
anvil watch --source                     # Watch source files
anvil watch --plans                      # Watch planning documents
anvil watch --all                        # Watch everything
anvil watch --debounce 500               # Custom debounce (ms, default: 300)
anvil watch --exclude "vendor/**,tmp/**" # Exclude matching glob paths
anvil watch --patterns "**/*.ts,**/*.rs" # Limit the watch loop to matching globs
anvil watch --file src/api/              # Scope to specific path
anvil watch                              # Run code-quality checks on each change (default)
anvil watch --action gate                # Run gate on each change instead
anvil watch --action none                # Architecture/dependency watch only, no code scan
```

| Flag         | Short | Default | Description                                                                         |
| ------------ | ----- | ------- | ----------------------------------------------------------------------------------- |
| `--source`   |       | —       | Watch source files (`src/**/*.ts`, `src/**/*.tsx`, `lib/**/*.ts`, `crates/**/*.rs`) |
| `--plans`    |       | —       | Watch plan files (`**/*.md`, `**/*.aps.md`, `**/prd.*`, `**/plan.*`, `**/spec.*`)   |
| `--all`      |       | —       | Watch all file types (source + plans)                                               |
| `--debounce` |       | `300`   | Milliseconds to wait before re-checking                                             |
| `--exclude`  |       | —       | Comma-separated glob patterns to exclude from watch events                          |
| `--patterns` |       | —       | Comma-separated glob patterns to include in watch events                            |
| `--file`     | `-f`  | —       | Scope watch to a specific file or directory                                         |
| `--action`   | `-a`  | `check` | Action to run on change: `check` (default), `gate`, or `none` (architecture-only)   |

Bare names match only that exact path. To exclude a directory's contents, use a
glob such as `vendor/**` rather than `vendor`.

### Save-time validation through the daemon

The full save-time story — daemon role, assurance states, confinement, and
fallback behaviour — lives in the
[save-time validation guide](../guides/save-time-validation.md). This section is
the configuration reference for the routing control.

By default `anvil watch` uses daemon-backed save-time validation when a resident
intercept daemon is already live and serving the save-time verbs. The daemon
validates the changed-path delta against one warm model rather than spawning a
per-save subprocess, so `anvil watch` and the editor/agent MCP
`anvil_validate_write` tool converge on the same verdict path. As of
`v0.8.2-beta`, an interactive `anvil start` auto-starts the daemon and an
interactive `anvil watch` offers to start one when none is answering (Linux and
macOS) — pass `--no-daemon`, or set `ANVIL_NO_DAEMON=1` for `start`, to suppress
that auto-start/offer (a daemon already running is still reused). See the
[daemon lifecycle](../guides/save-time-validation.md#daemon-lifecycle) for the
full start/offer/opt-out model.

If no daemon is live, the default routing path stays quiet and runs a scoped
`check` over exactly the changed files, never a whole-repository `--all` scan.
Set `ANVIL_WATCH_DAEMON=0` (also `false`/`off`/`no`) to opt out of daemon
routing. Set `ANVIL_WATCH_DAEMON=1` (also `true`/`on`/`yes`) to force daemon
routing for diagnostics; in that forced mode, an absent daemon falls back to the
same scoped `check` and reports save-time assurance as `unavailable` rather than
a misleading `clean`. Value matching is case-insensitive. Any other value —
including an empty `ANVIL_WATCH_DAEMON=` — carries no explicit opinion and is
treated as unset (the safe default-on-when-live posture); opt out with an
explicit false value, not by blanking.

When daemon routing is active, `anvil status` gains a `Save-time:` line
reporting the current assurance state (`clean`, `stale`, `pending`, `running`,
`bounded`, or `unavailable`) and, in confined mode, the size of the
admitted-workspace allow-list. With the variable unset and no daemon live,
`anvil status` prints an explicit off-state line naming `anvil start`; only an
explicit `ANVIL_WATCH_DAEMON=0` opt-out hides the save-time line.

### Implicit background scans

The daemon now warms a worktree's graph **opportunistically** on first contact:
`anvil status` (`workspace_status`) and the GCTX assistant queries spawn a
background full scan when they hit a cold key, so the next query need not wait
for a manual save. This means `workspace_status` went from a read-only probe to
a scan-triggering call.

Set `ANVIL_WATCH_DAEMON_SCAN=0` to disable **only** that implicit
background-scan trigger while keeping the daemon serving — reads still answer
from whatever warm state exists, but a cold key is no longer auto-warmed by a
probe. This is the scoped lever for operators who want the daemon's read surface
without the first-contact scan cost; `ANVIL_WATCH_DAEMON=0` remains the
full-bypass lever (no daemon routing at all). An **explicit** `anvil scan` /
`request_full_scan` is never suppressed by `ANVIL_WATCH_DAEMON_SCAN=0` — only
the opportunistic auto-warm. The value is trimmed before matching, so `" 0"` /
`"0\n"` also disable; any other value (or unset) leaves the auto-warm on.

## Workspace confinement

The intercept daemon serves save-time validation for a set of workspace roots.
By default it runs in **open** mode and adopts each repository on first touch.
For shared or multi-tenant machines you can confine it to an explicit allow-list
so it only serves roots you admit. Confinement is operator config the daemon
reads live — no restart is required.

```bash
anvil workspace list                      # Show the current mode and allow entries
anvil workspace mode allowlist            # Only serve the configured allow
                                          # entries (empty list admits nothing)
anvil workspace allow /path/to/repo       # Admit one root (exact match)
anvil workspace allow /srv/work --prefix  # Admit an entire subtree
anvil workspace deny /path/to/repo        # Remove an allow entry
anvil workspace mode open                 # Back to first-touch adopt (the default)
```

In `allowlist` mode the daemon admits **exactly** the configured allow entries
and nothing implicit — an empty allow-list admits no roots (fail-closed), so add
the roots you want served with `anvil workspace allow <path>` before switching a
shared machine into allowlist mode. `anvil status` shows `· confined: <N>` next
to the save-time line when the daemon is in allowlist mode.

### Register a worktree on every startup

Confinement (`allow`) is the set of roots the daemon **may** serve; registration
is the set of worktrees it **actively** protects. These are deliberately
distinct. `anvil workspace register <path>` registers a worktree for durable
protection now, but a worktree only re-registers automatically across daemon
restarts if it is in the `register_on_start` list:

```bash
anvil workspace register /path/to/repo --persist   # register now AND on every startup
anvil workspace register --all --persist           # register allow-listed worktrees; --persist records register_on_start
anvil workspace unregister /path/to/repo --persist  # stop re-registering it on startup
anvil workspace list                                # shows allow entries, live registry,
                                                    # and the register_on_start set
```

`--persist` records the worktree under a `register_on_start:` key in
`workspace.yaml`; the daemon registers those worktrees as durable members when
it starts, before accepting connections. Entries whose directory is gone are
skipped and reported. There is **no filesystem scan** — only the exact paths you
list are touched.

> **Registration does not grant admission.** In `allowlist` mode a registered
> worktree (including one in `register_on_start`) is **not** implicitly admitted
> — admission is decided solely by the allow entries. If you want a registered
> worktree served under confinement, add it with `anvil workspace allow <path>`
> too. `register --all --persist` records successfully registered worktree roots
> in `register_on_start` (re-registration intent); it does not modify allow
> entries — add those separately with `anvil workspace allow <path>` if needed.

> **Format version & downgrade safety.** Writing a `register_on_start` entry
> bumps the config to format `version: 2`. A daemon **newer than or equal to**
> the one that wrote the file reads it; a daemon that encounters a config at a
> higher format version than it understands ignores keys it does not know rather
> than failing closed, so a later schema addition can never collapse the
> confinement trust floor on a version skew. A pure-confinement config (no
> `register_on_start`) stays at the implicit version 1 and is byte-compatible
> with pre-`register_on_start` daemons. Because `register_on_start` is opt-in,
> only a worktree you explicitly persist is ever affected.
>
> The one direction that is **not** safe is downgrading the Anvil **binary**
> below the version that wrote a `version: 2` file: a pre-`register_on_start`
> daemon does not understand `version: 2` and fails the whole config closed — in
> `allowlist` mode it then admits **no** roots until you either upgrade Anvil
> again or remove the `register_on_start` key by hand. This only affects
> machines where you both used `--persist` and rolled the binary back; new
> schema additions from here on are handled by the forward-compat rule above and
> never trigger it.

### Auto-register newly-created worktrees

Git has no native post-`worktree add` hook, so Anvil cannot transparently
register a worktree the moment you create it. Instead, `install-hook` adds a
guided Git alias you opt into:

```bash
anvil workspace install-hook            # installs `git wt-add`
anvil workspace install-hook --print    # print the alias + PowerShell form, install nothing
git wt-add ../my-worktree               # = git worktree add … then anvil workspace register
```

The alias is a portable POSIX `sh` one-liner (it runs through Git's bundled `sh`
on every platform, including Git-for-Windows); on Windows the command also
prints a PowerShell `$PROFILE` function you can use instead. It registers the
**first** operand of `git worktree add` (the worktree path), skipping flags and
a `-b <branch>` value, so `git wt-add -b feature ../wt main` registers `../wt`.
It never shims `git` — only an alias you invoke by name.

## CI Mode

Use gate profiles for CI environments:

```bash
anvil gate --profile ci           # All checks, plain output
anvil gate --profile dev          # Skips coverage and dependency checks
anvil gate --profile production   # All checks
anvil gate --list-profiles        # Show available profiles
```

| Profile      | Skips                            | Use case                     |
| ------------ | -------------------------------- | ---------------------------- |
| `dev`        | coverage, dependency             | Local development            |
| `ci`         | (none)                           | CI pipelines                 |
| `production` | (none)                           | Release validation           |
| `ai`         | lint, test, coverage, dependency | AI guardrail / agent surface |

The `ai` profile selects the curated AI guardrail check set, treats missing or
invalid governance config as blocking, and emits the canonical
`anvil.diagnostic.v1` JSON envelope by default so agent and MCP consumers can
parse results without bespoke flag plumbing.

Additional runtime flags:

```bash
anvil gate --skip-checks "coverage,dependency"                 # Skip specific checks
anvil gate --only-checks "secret-detection,import-boundaries" # Run only specific checks
anvil gate --fail-fast                           # Stop on first failure
anvil gate --progress                            # Show real-time progress
anvil --json gate                                # JSON output (global flag)
```

:::note

`--json` is a **global flag** that must appear before the subcommand:
`anvil --json gate`, not `anvil gate --json`.

:::

## Git Hooks

`anvil hooks install` installs file-mode Git hooks under `.git/hooks/` (the
default). On Git 2.54 or newer you can opt into config-mode hooks instead, which
appends Anvil-owned `hook.<event>.command` entries to your local Git config
without writing files:

```bash
anvil hooks install --config
anvil hooks uninstall --config
anvil hooks status
```

`anvil hooks status` and `anvil doctor` detect file-mode hooks, config-mode
hooks, third-party hook managers (Husky, Lefthook, pre-commit), and
`core.hooksPath` overrides, and warn when the same event would fire twice. Husky
remains the recommended contributor bootstrap inside this repository; `--config`
is an explicit opt-in for power users.

## Environment Variables

:::note

The Rust CLI does not support environment variables for selecting or configuring
`.anvilrc`, gate checks, or other project configuration. Use CLI flags and
config files for those settings.

The Rust CLI does read some environment variables for auth and API-related
configuration, including:

- `ANVIL_API_URL` — custom API endpoint
- `ANVIL_LICENSE` — licence key for CI environments
- `ANVIL_ADMIN_KEY` — admin command authentication
- `ANVIL_TEMPLATES_DIR` — custom template directory
- `ANVIL_SCAN_THREADS` — cap on the parallel-scan thread pool used by first-run
  scans, `check`, `gate`, and `audit` (default `min(num_cpus, 4)`); raise this
  when running on a dedicated CI runner
- `ANVIL_WATCH_DAEMON` — unset defaults to daemon-backed `anvil watch` routing
  only when a live daemon answers; set `0` (or `false`/`off`/`no`) to opt out
  (no routing, reuse, start, or offer), or `1` (or `true`/`on`/`yes`) to force
  routing for diagnostics. See
  [Save-time validation through the daemon](#save-time-validation-through-the-daemon)
- `ANVIL_WATCH_DAEMON_SCAN` — set `0` to disable only the **implicit**
  background-scan trigger (the first-contact auto-warm fired by `anvil status` /
  GCTX queries on a cold key) while keeping the daemon serving; an explicit
  `request_full_scan` is never suppressed. See
  [Implicit background scans](#implicit-background-scans)
- `ANVIL_NO_DAEMON` — set to a non-empty value to stop `anvil start` from
  auto-starting the per-user save-time daemon (the environment equivalent of
  `--no-daemon`); a daemon already running is still reused. See the
  [daemon lifecycle](../guides/save-time-validation.md#daemon-lifecycle)
- `ANVIL_USAGE_DISABLE` — set `1` to decline local command-invocation usage
  collection; the CLI `command.invoked` producer writes nothing.
  `DO_NOT_TRACK=1` is honoured as an alias, and the whole-observation
  break-glass `ANVIL_INTERCEPT_DISABLE_OBSERVATION=1` disables both the CLI and
  daemon usage producers. See the
  `docs/observability/usage-analytics.md#operator-controls-environment-variables`
- `ANVIL_USAGE_SIDECAR_NO_TRIM` — any non-empty value disables the lazy 7-day /
  64 MiB retention trim on the usage sidecar (the sidecar then grows unbounded)
- `ANVIL_OBSERVATION_INCLUDE_PATHS` — set `1` to make the daemon save-time /
  fence observation rows record absolute validated paths instead of only a path
  count. **Changes the privacy posture; off by default.** See the
  `docs/observability/usage-analytics.md#operator-controls-environment-variables`
- `ANVIL_TRACK_SURFACE_SQL`, `ANVIL_TRACK_SURFACE_GHA`,
  `ANVIL_TRACK_SURFACE_DOCK`, `ANVIL_TRACK_SURFACE_SH` — set `0` to opt out of
  the matching default-on infrastructure-hygiene surface for the session, or `1`
  to force it on.

Legacy Node.js environment variables (`ANVIL_CI`, `ANVIL_FAIL_ON_WARNINGS`) are
not supported.

:::

## Exit Codes

| Code | Meaning         | Typical action         |
| ---- | --------------- | ---------------------- |
| 0    | All checks pass | Continue               |
| 1    | General error   | Investigate            |
| 2    | Gate failure    | Block merge            |
| 3    | Auth required   | Run `anvil auth login` |
| 4    | Config error    | Fix `.anvilrc`         |

---

**Next:** [Security model →](/anvil/operations/security)
