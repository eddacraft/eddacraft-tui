---
id: architecture
title: Architecture Boundaries
sidebar_position: 3
---

# Architecture Boundaries

anvil enforces module boundaries by analysing import graphs. In Anvil's quality
model, architecture is one family of checks: the check reads your project graph,
surfaces boundary findings, and contributes those findings to the gate result.
This tutorial covers defining layers, writing an architecture file, and
validating boundaries.

## Prerequisites

- anvil initialised (`anvil init`)
- A TypeScript or JavaScript project with at least a few directories under
  `src/`

## 1. Plan Your Layers

Before writing config, decide which directories form distinct layers. Common
patterns:

| Pattern       | Layers                                                      |
| ------------- | ----------------------------------------------------------- |
| **Starter**   | `components`, `lib`, `services`                             |
| **Layered**   | `presentation`, `business`, `data`, `shared`                |
| **Hexagonal** | `core`, `ports`, `adapters`, `application`                  |
| **Clean**     | `entities`, `use_cases`, `interface_adapters`, `frameworks` |
| **DDD**       | `domain`, `application`, `infrastructure`, `interfaces`     |
| **Monorepo**  | `apps`, `packages`, `shared`                                |

## 2. Create the Architecture File

Create `.anvil/architecture.yaml` in your project root. Here is a layered
example:

```yaml
schema_version: '0.1.0'
template: layered
layers:
  presentation:
    patterns:
      - 'src/api/**'
    depends_on:
      - business
      - shared

  business:
    patterns:
      - 'src/services/**'
    depends_on:
      - data
      - shared

  data:
    patterns:
      - 'src/repositories/**'
    depends_on:
      - shared

  shared:
    patterns:
      - 'src/utils/**'
    depends_on: []
```

Edit the `patterns` values to match your actual directory layout.

## 3. Validate the Architecture Definition

Check that your architecture file is well-formed:

```bash
anvil architecture validate
```

If valid, anvil confirms the layer count and dependency rules. Fix any errors
before proceeding.

## 4. Review the Loaded Architecture

View the architecture as anvil sees it:

```bash
anvil architecture show
```

This prints the resolved layers, allowed/denied imports, and file counts.

## 5. Check Your Code Against Boundaries

This command runs the import-boundary gate across your codebase. In this
tutorial, the relevant findings come from the architecture check family.

```bash
anvil gate --only-checks import-boundaries
```

```
Checking architecture...
  ARCH-001: Boundary violation
    src/api/handlers/report.ts:3
    imports from ../../repositories/report.repo
    Rule: presentation denies imports from data

1 architecture violation found.
```

In the sample output above, `Boundary violation` is an architecture finding. The
gate can later use that finding when deciding whether the workflow passes.

## 6. Fix or Suppress

**Fix** -- route the import through the correct layer:

```typescript
// Before (violation)
import { ReportRepo } from '../../repositories/report.repo';

// After (correct)
import { ReportService } from '../../services/report.service';
```

**Suppress** -- if the violation is intentional:

```typescript
// @anvil-ignore ARCH-001 Direct access needed for bulk export job
import { ReportRepo } from '../../repositories/report.repo';
```

:::caution

Suppressions require a reason. A bare `@anvil-ignore` without an explanation
triggers its own warning.

:::

---

**Next:** [Drift Detection](/anvil/tutorials/drift)
