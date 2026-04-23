---
id: config
title: Configuration
description: Complete reference for anvil configuration options.
sidebar_position: 1
---

# Configuration

anvil uses two configuration files and CLI flags for runtime options.

## Configuration Files

| File                       | Purpose                                         |
| -------------------------- | ----------------------------------------------- |
| `.anvilrc`                 | Project-level settings (checks, format, paths)  |
| `.anvil/gate-config.json`  | Checks used by `anvil gate` and gate thresholds |
| `.anvil/architecture.yaml` | Architecture layer and boundary definitions     |

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

## Gate Configuration

Managed by `anvil gate-config`. Stored at `.anvil/gate-config.json`.

Use `anvil gate-config --list` to view the current configuration, and
`--enable <check>` / `--disable <check>` to toggle individual checks.

Use this file to control which checks feed the gate and what threshold the gate
uses when it summarises the overall result.

`.anvilrc` sets your project's default analysis checks. `gate-config` controls
the broader gate run, including build-and-CI checks such as `lint`, `test`,
`coverage`, and `dependency` alongside Anvil analysis checks such as
`secret-detection`, `import-boundaries`, `antipattern-scan`, and `policy`.

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

Validate with `anvil architecture validate` and inspect with
`anvil architecture show`.

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
| `DD-001` | deferred-debt             | TODO/FIXME without tracking reference    | warning  |
| `DD-002` | deferred-debt             | HACK comment without tracking reference  | warning  |
| `DD-003` | deferred-debt             | Temporary code without expiry            | info     |
| `DD-004` | deferred-debt             | Completion claim with outstanding TODOs  | warning  |

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

## Watch Mode

Watch mode is configured via CLI flags, not config files.

```bash
anvil watch --source                     # Watch source files
anvil watch --plans                      # Watch planning documents
anvil watch --all                        # Watch everything
anvil watch --debounce 500               # Custom debounce (ms, default: 300)
anvil watch --exclude "vendor,tmp"       # Exclude directories
anvil watch --patterns "**/*.ts,**/*.rs" # Custom file patterns
anvil watch --file src/api/              # Scope to specific path
anvil watch --action gate                # Run gate on each change
```

| Flag         | Short | Default | Description                                                                         |
| ------------ | ----- | ------- | ----------------------------------------------------------------------------------- |
| `--source`   |       | —       | Watch source files (`src/**/*.ts`, `src/**/*.tsx`, `lib/**/*.ts`, `crates/**/*.rs`) |
| `--plans`    |       | —       | Watch plan files (`**/*.md`, `**/*.aps.md`, `**/prd.*`, `**/plan.*`, `**/spec.*`)   |
| `--all`      |       | —       | Watch all file types (source + plans)                                               |
| `--debounce` |       | `300`   | Milliseconds to wait before re-checking                                             |
| `--exclude`  |       | —       | Comma-separated directory names to skip                                             |
| `--patterns` |       | —       | Comma-separated glob patterns to watch                                              |
| `--file`     | `-f`  | —       | Scope watch to a specific file or directory                                         |
| `--action`   | `-a`  | —       | Action to run on change: `gate` or `check`                                          |

## CI Mode

Use gate profiles for CI environments:

```bash
anvil gate --profile ci           # All checks, plain output
anvil gate --profile dev          # Skips coverage and dependency checks
anvil gate --profile production   # All checks
anvil gate --list-profiles        # Show available profiles
```

| Profile      | Skips                | Use case           |
| ------------ | -------------------- | ------------------ |
| `dev`        | coverage, dependency | Local development  |
| `ci`         | (none)               | CI pipelines       |
| `production` | (none)               | Release validation |

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

Legacy Node.js environment variables (`ANVIL_CI`, `ANVIL_FAIL_ON_WARNINGS`) are
not supported.

:::

## Exit Codes

| Code | Meaning         | Typical action    |
| ---- | --------------- | ----------------- |
| 0    | All checks pass | Continue          |
| 1    | General error   | Investigate       |
| 2    | Gate failure    | Block merge       |
| 3    | Auth required   | Run `anvil login` |
| 4    | Config error    | Fix `.anvilrc`    |

---

**Next:** [Security model →](/anvil/operations/security)
