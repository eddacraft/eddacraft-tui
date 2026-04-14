---
id: first-project
title: First Project
description: Set up anvil in an existing project with architecture boundaries.
sidebar_position: 4
---

# First Project

This guide walks through setting up anvil in an existing TypeScript project with
intentional architecture boundaries.

## Scenario

You have a project with this structure:

```
my-app/
├── src/
│   ├── api/           # HTTP handlers
│   ├── services/      # Business logic
│   ├── repositories/  # Data access
│   └── utils/         # Shared utilities
├── package.json
└── tsconfig.json
```

You want to enforce these boundaries:

- `api/` can import from `services/` but not `repositories/`
- `services/` can import from `repositories/` and `utils/`
- `repositories/` can only import from `utils/`
- `utils/` cannot import from other layers

## Step 1: Install and Initialise

```bash
# macOS / Linux
curl -fsSL https://install.eddacraft.ai | sh
```

```powershell
# Windows (PowerShell)
irm https://install.eddacraft.ai/windows | iex
```

Then initialise in your project:

```bash
anvil init
```

## Step 2: Define Architecture Boundaries

Create `.anvil/architecture.yaml` to define your layer rules. Layers are a map
keyed by name, with `patterns` (glob list) and `depends_on` (allowed
dependencies):

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

The `depends_on` field declares which layers a given layer **may** import from.
Any import outside that list is a boundary violation.

Validate the definition:

```bash
anvil architecture validate
```

## Step 3: Run Initial Check

```bash
anvil check --all
```

You might see existing violations:

```
Checking architecture...
  ARCH-001: Boundary violation
    src/api/handlers/user.ts imports from src/repositories/user.repo.ts
    Rule: api-layer denies imports from src/repositories/**

Checking anti-patterns...
  [AP-003] Explicit 'any' type
    src/services/parser.ts:42:10

1 error, 1 warning found.
Gate status: FAIL
```

## Step 4: Fix Violations

For architecture violations, you have two options:

### Option A: Fix the Code

Refactor to respect boundaries. In the example above, the API handler should
call a service, not a repository directly:

```typescript
// Before (violation)
import { UserRepo } from '../repositories/user.repo';

// After (correct)
import { UserService } from '../services/user.service';
```

### Option B: Suppress with Explanation

If the violation is intentional, add a suppression:

```typescript
// @anvil-ignore ARCH-001: Legacy pattern, will refactor in Q2
import { UserRepo } from '../repositories/user.repo';
```

:::caution

Suppressions require explanations. `@anvil-ignore` without a reason will itself
trigger a warning.

:::

## Step 5: Start Watch Mode

Once clean (or suppressions added), start watch mode:

```bash
anvil watch
```

Now any new violations will surface immediately when you save.

## Step 6: Add to CI

Add anvil to your CI pipeline:

```yaml
# .github/workflows/ci.yml (Linux runner)
- name: Install anvil
  run: curl -fsSL https://install.eddacraft.ai | sh

- name: Run anvil
  run: anvil gate --profile ci
```

For Windows runners:

```yaml
# .github/workflows/ci.yml (Windows runner)
- name: Install anvil
  shell: pwsh
  run: irm https://install.eddacraft.ai/windows | iex

- name: Run anvil
  run: anvil gate --profile ci
```

The `ci` profile runs all checks and produces machine-readable output with
appropriate exit codes.

## What You've Achieved

- anvil validates architecture boundaries on every save
- New boundary violations are caught before commit
- Anti-patterns surface immediately
- CI blocks PRs with violations

---

**Previous:** [Quickstart](/anvil/quickstart) | **Next:**
[Experience your first gate moment →](/anvil/first-gate)

**Learn more:**

- [Custom policies](/anvil/tutorials/policies) -- write OPA/Rego rules for your
  standards
- [Architecture boundaries](/anvil/tutorials/architecture) -- templates and
  enforcement
- [Drift detection](/anvil/tutorials/drift) -- track architectural changes
- [CI integration](/anvil/tutorials/ci) -- add anvil to your pipeline
- [Suppressions](/anvil/tutorials/suppressions) -- manage legacy warnings
