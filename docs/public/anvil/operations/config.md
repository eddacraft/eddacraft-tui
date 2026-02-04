---
id: config
title: Configuration
description: Complete reference for Anvil configuration options.
sidebar_position: 1
---

# Configuration

Anvil is configured via `anvil.config.json` in your project root.

## Basic Structure

```json
{
  "version": "1.0",
  "gates": {
    "architecture": { ... },
    "antiPatterns": { ... },
    "secrets": { ... },
    "eslint": { ... },
    "coverage": { ... }
  },
  "watch": { ... },
  "ci": { ... },
  "suppressions": [ ... ]
}
```

## Gates

### Architecture

```json
{
  "architecture": {
    "enabled": true,
    "boundaries": [
      {
        "name": "api-layer",
        "pattern": "src/api/**",
        "allow": ["src/services/**", "src/utils/**"],
        "deny": ["src/repositories/**"]
      }
    ],
    "detectCircular": true,
    "maxDepth": 10
  }
}
```

| Option                 | Type    | Description                    |
| ---------------------- | ------- | ------------------------------ |
| `enabled`              | boolean | Enable architecture checks     |
| `boundaries`           | array   | Boundary definitions           |
| `boundaries[].name`    | string  | Human-readable name            |
| `boundaries[].pattern` | glob    | Files this boundary applies to |
| `boundaries[].allow`   | glob[]  | Allowed import patterns        |
| `boundaries[].deny`    | glob[]  | Forbidden import patterns      |
| `detectCircular`       | boolean | Detect circular dependencies   |
| `maxDepth`             | number  | Max import chain depth         |

### Anti-Patterns

```json
{
  "antiPatterns": {
    "enabled": true,
    "patterns": {
      "AP-001": { "enabled": true, "severity": "warning" },
      "AP-003": { "enabled": true, "severity": "error" },
      "AP-004": { "enabled": true, "severity": "warning" },
      "AP-006": { "enabled": true, "severity": "warning" },
      "AP-007": { "enabled": false }
    }
  }
}
```

| Pattern  | Description            | Default Severity |
| -------- | ---------------------- | ---------------- |
| `AP-001` | Broad `eslint-disable` | warning          |
| `AP-003` | Explicit `any` type    | warning          |
| `AP-004` | `@ts-ignore` directive | warning          |
| `AP-006` | Empty catch block      | warning          |
| `AP-007` | Console in production  | info (off)       |

### Secrets

```json
{
  "secrets": {
    "enabled": true,
    "patterns": ["api[_-]?key", "secret[_-]?key", "password"],
    "entropyThreshold": 4.5,
    "checkGitHistory": false,
    "allowList": ["src/config/example.ts"]
  }
}
```

| Option             | Type    | Description                     |
| ------------------ | ------- | ------------------------------- |
| `enabled`          | boolean | Enable secret detection         |
| `patterns`         | regex[] | Additional patterns to match    |
| `entropyThreshold` | number  | Shannon entropy threshold (0-8) |
| `checkGitHistory`  | boolean | Scan git history                |
| `allowList`        | glob[]  | Files to skip                   |

### ESLint

```json
{
  "eslint": {
    "enabled": true,
    "failOn": "error",
    "configPath": ".eslintrc.js"
  }
}
```

### Coverage

```json
{
  "coverage": {
    "enabled": false,
    "threshold": {
      "lines": 80,
      "branches": 75,
      "functions": 80,
      "statements": 80
    },
    "reportPath": "coverage/coverage-summary.json"
  }
}
```

## Watch Mode

```json
{
  "watch": {
    "enabled": true,
    "debounceMs": 300,
    "ignore": ["node_modules", "dist", ".git", "coverage"],
    "extensions": [".ts", ".tsx", ".js", ".jsx"]
  }
}
```

| Option       | Type     | Description              |
| ------------ | -------- | ------------------------ |
| `debounceMs` | number   | Delay before validation  |
| `ignore`     | glob[]   | Patterns to ignore       |
| `extensions` | string[] | File extensions to watch |

## CI Mode

```json
{
  "ci": {
    "failOnWarnings": false,
    "outputFormat": "json",
    "evidencePath": ".anvil/evidence"
  }
}
```

## Suppressions

```json
{
  "suppressions": [
    {
      "pattern": "src/legacy/**",
      "checks": ["AP-003", "AP-006"],
      "reason": "Legacy code, migration planned Q2"
    },
    {
      "pattern": "src/generated/**",
      "checks": ["*"],
      "reason": "Auto-generated from protobuf"
    }
  ]
}
```

## Evidence

```json
{
  "evidence": {
    "enabled": true,
    "path": ".anvil/evidence",
    "retention": {
      "duration": "90d",
      "keepFailures": true,
      "compressAfter": "7d"
    }
  }
}
```

## Extending Configuration

### Local Overrides

Create `anvil.local.json` (gitignored):

```json
{
  "extends": "./anvil.config.json",
  "watch": {
    "debounceMs": 500
  }
}
```

### Environment Variables

Override via environment:

```bash
ANVIL_FAIL_ON_WARNINGS=true anvil run
```

| Variable                 | Description            |
| ------------------------ | ---------------------- |
| `ANVIL_CONFIG`           | Path to config file    |
| `ANVIL_CI`               | Force CI mode          |
| `ANVIL_FAIL_ON_WARNINGS` | Fail on warnings       |
| `ANVIL_DISABLE`          | Disable Anvil entirely |

## Full Example

```json
{
  "version": "1.0",
  "gates": {
    "architecture": {
      "enabled": true,
      "boundaries": [
        {
          "name": "api",
          "pattern": "src/api/**",
          "deny": ["src/repositories/**"]
        },
        {
          "name": "services",
          "pattern": "src/services/**",
          "allow": ["src/repositories/**"]
        }
      ]
    },
    "antiPatterns": {
      "enabled": true,
      "patterns": {
        "AP-001": { "severity": "error" },
        "AP-003": { "severity": "error" },
        "AP-004": { "severity": "warning" },
        "AP-006": { "severity": "warning" }
      }
    },
    "secrets": {
      "enabled": true,
      "entropyThreshold": 4.5
    }
  },
  "watch": {
    "debounceMs": 300,
    "ignore": ["node_modules", "dist"]
  },
  "ci": {
    "failOnWarnings": false
  },
  "suppressions": [
    {
      "pattern": "scripts/**",
      "checks": ["AP-007"],
      "reason": "Scripts may use console"
    }
  ]
}
```

---

**Next:** [Security model →](/docs/anvil/operations/security)
