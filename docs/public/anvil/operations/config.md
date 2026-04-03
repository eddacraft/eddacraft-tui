---
id: config
title: Configuration
description: Complete reference for Anvil configuration options.
sidebar_position: 1
---

# Configuration

Anvil uses two configuration files and CLI flags for runtime options.

## Configuration Files

| File                       | Purpose                                        |
| -------------------------- | ---------------------------------------------- |
| `.anvilrc`                 | Project-level settings (checks, format, paths) |
| `.anvil/gate-config.json`  | Gate check definitions and thresholds          |
| `.anvil/architecture.yaml` | Architecture layer and boundary definitions    |

## `.anvilrc`

Created by `anvil init`. Supports JSON, YAML, and TOML formats.

### JSON (default)

```json
{
  "schemaVersion": "1.0.0",
  "planningDir": "plans",
  "format": "yaml",
  "checks": ["secret-detection", "import-boundaries"]
}
```

### YAML

```yaml
schemaVersion: '1.0.0'
planningDir: plans
format: yaml
checks:
  - secret-detection
  - import-boundaries
```

### TOML

```toml
schema_version = "1.0.0"
planning_dir = "plans"
format = "yaml"
checks = ["secret-detection", "import-boundaries"]
```

:::note

JSON and YAML use **camelCase** keys. TOML uses **snake_case** keys.

:::

| Field           | Type     | Default                                     | Description                         |
| --------------- | -------- | ------------------------------------------- | ----------------------------------- |
| `schemaVersion` | string   | `"1.0.0"`                                   | Config schema version               |
| `planningDir`   | string   | `"plans"`                                   | Directory for APS plan files        |
| `format`        | string   | `"yaml"`                                    | Plan format: `json`, `yaml`, `toml` |
| `checks`        | string[] | `["secret-detection", "import-boundaries"]` | Enabled project checks              |

### Available Checks

| Check               | Description                           |
| ------------------- | ------------------------------------- |
| `secret-detection`  | Detect leaked secrets and credentials |
| `import-boundaries` | Enforce module import boundaries      |
| `antipattern-scan`  | Detect common code anti-patterns      |
| `architecture`      | Validate architecture definitions     |
| `policy`            | Evaluate OPA policy rules             |

## Gate Configuration

Managed by `anvil gate-config`. Stored at `.anvil/gate-config.json`.

Use `anvil gate-config --list` to view the current configuration, and
`--enable <check>` / `--disable <check>` to toggle individual checks.

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
      "name": "secret",
      "description": "Secret and credential detection",
      "enabled": true
    },
    {
      "name": "architecture",
      "description": "Architecture boundary validation",
      "enabled": true
    },
    {
      "name": "policy",
      "description": "Policy compliance evaluation",
      "enabled": true
    }
  ],
  "thresholds": {
    "overall_score": 80
  }
}
```

Each check can have an optional `config` object for check-specific settings.

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

The `schema_version` field must be exactly `"0.1.0"`. Anvil validates this on
every run and rejects definitions with a different version.

:::

### Templates

Use `template` to start from a preset layer structure. Anvil fills in default
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

Anti-pattern detection is configured per-pattern. There are 13 built-in
patterns: 4 enabled by default, 9 opt-in.

### Default Patterns (always active)

| Pattern  | Description            | Severity |
| -------- | ---------------------- | -------- |
| `AP-001` | Broad `eslint-disable` | warning  |
| `AP-003` | Explicit `any` type    | warning  |
| `AP-004` | `@ts-ignore` directive | warning  |
| `AP-006` | Empty catch block      | warning  |

### Opt-in Patterns

Enable with `anvil check --include-opt-in`:

| Pattern  | Description                    | Severity |
| -------- | ------------------------------ | -------- |
| `AP-002` | Rule-specific `eslint-disable` | info     |
| `AP-005` | `@ts-expect-error` directive   | info     |
| `AP-007` | Console in production code     | info     |
| `AP-008` | Inline `style` attribute       | warning  |
| `AP-009` | Inline `<script>` block        | warning  |
| `AP-010` | Inline event handler           | warning  |
| `AP-011` | Deprecated HTML tag            | warning  |
| `AP-012` | `!important` in CSS            | warning  |
| `AP-013` | CSS `@import`                  | info     |

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
// @anvil-ignore AP-003 Legacy parser uses any, migration planned Q2
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
anvil gate --skip-checks "coverage,dependency"   # Skip specific checks
anvil gate --only-checks "secret,architecture"   # Run only specific checks
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

The Rust CLI does not currently read environment variables for configuration.
Use CLI flags and config files instead. Legacy Node.js environment variables
(`ANVIL_CI`, `ANVIL_FAIL_ON_WARNINGS`) are not supported.

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
