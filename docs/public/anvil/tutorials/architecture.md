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
- A TypeScript, JavaScript, or Rust project with at least a few directories
  under `src/`

The commands in this tutorial are the same on macOS, Linux, and Windows. On
Windows, run them from PowerShell in the project root; keep the YAML patterns
with forward slashes (`src/services/**`) because anvil treats them as
workspace-relative globs, not native shell paths.

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

The same file shape works for a Rust crate — layers map module directories under
the crate's `src/`:

```yaml
schema_version: '0.1.0'
template: layered
layers:
  api:
    patterns:
      - 'src/api/**'
    depends_on:
      - services
      - shared

  services:
    patterns:
      - 'src/services/**'
    depends_on:
      - storage
      - shared

  storage:
    patterns:
      - 'src/storage/**'
    depends_on:
      - shared

  shared:
    patterns:
      - 'src/util/**'
    depends_on: []
```

For Rust, boundary edges come from `use crate::…` / `self::…` / `super::…`
imports resolved within each crate's `src/` tree. External crates (`std`,
`serde`, …) are never boundary findings, and an import anvil cannot resolve to a
file is dropped rather than guessed — a missed edge is a missed drift signal,
never a false violation.

## 3. Validate the Architecture Definition

Check that your architecture file is well-formed:

macOS / Linux:

```bash
anvil architecture validate
```

Windows PowerShell:

```powershell
anvil architecture validate
```

If valid, anvil confirms the layer count and dependency rules. Fix any errors
before proceeding.

## 4. Review the Loaded Architecture

View the architecture as anvil sees it:

macOS / Linux:

```bash
anvil architecture show
```

Windows PowerShell:

```powershell
anvil architecture show
```

This prints the resolved layers, allowed/denied imports, and file counts.

## 5. Check Your Code Against Boundaries

This command runs the import-boundary gate across your codebase. In this
tutorial, the relevant findings come from the architecture check family.

macOS / Linux:

```bash
anvil gate --only-checks import-boundaries
```

Windows PowerShell:

```powershell
anvil gate --only-checks import-boundaries
```

```
Checking import-boundaries...
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

The Rust equivalent:

```rust
// Before (violation): api reaches into storage directly
use crate::storage::report_store::ReportStore;

// After (correct): route through the service layer
use crate::services::report_service::ReportService;
```

**Accept** -- if the cross-layer import is intentional, record the decision
where the checker reads it:

- **Allow it in the architecture file** -- add the target layer to the importing
  layer's `depends_on`. The architecture file is the contract; an intended
  dependency belongs in it, not beside the import.
- **Rely on the baseline** -- anvil's posture is _new edges only_: edges
  captured in your baseline snapshot are existing posture and do not re-report;
  only new violations surface (see [Drift Detection](/anvil/tutorials/drift) for
  snapshots and budgets).

:::caution

Inline `// @anvil-ignore` comments suppress **antipattern** findings (the `AP-*`
/ `RS-*` rules) and are not read by the boundary checker — an
`@anvil-ignore ARCH-001` comment has no effect.

:::

---

**Next:** [Drift Detection](/anvil/tutorials/drift)
