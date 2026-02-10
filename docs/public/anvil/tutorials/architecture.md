---
id: architecture
title: Architecture Boundaries
sidebar_position: 3
---

# Architecture Boundaries

Anvil enforces module boundaries by analysing import graphs. This tutorial
covers project detection, template selection, and boundary validation.

## Prerequisites

- Anvil initialised (`anvil init`)
- A TypeScript or JavaScript project with at least a few directories under
  `src/`

## 1. Detect Project Structure

Anvil scans your source tree to identify existing modules:

```bash
anvil architecture detect
```

```
Detected modules:
  src/api/         (12 files)
  src/services/    (8 files)
  src/models/      (5 files)
  src/utils/       (3 files)
```

## 2. Choose a Template

Anvil ships with six architecture templates. Pick the one closest to your
project and customise from there.

| Template      | Layers                                                      |
| ------------- | ----------------------------------------------------------- |
| **Starter**   | `components`, `lib`, `services`                             |
| **Layered**   | `presentation`, `business`, `data`, `shared`                |
| **Hexagonal** | `core`, `ports`, `adapters`, `application`                  |
| **Clean**     | `entities`, `use_cases`, `interface_adapters`, `frameworks` |
| **DDD**       | `domain`, `application`, `infrastructure`, `interfaces`     |
| **Monorepo**  | `apps`, `packages`, `shared`                                |

## 3. Create Boundaries from a Template

```bash
anvil architecture create --template layered
```

```
Creating architecture from template: layered

Generated .anvil/architecture.yaml:
  presentation -> business  (allowed)
  business -> data          (allowed)
  business -> shared        (allowed)
  data -> shared            (allowed)

4 layers, 4 dependency rules.
```

## 4. Review the Generated File

The command produces `.anvil/architecture.yaml`:

```yaml
version: '1.0'
layers:
  - name: presentation
    pattern: 'src/api/**'
    allow:
      - business
      - shared
    deny:
      - data

  - name: business
    pattern: 'src/services/**'
    allow:
      - data
      - shared
    deny:
      - presentation

  - name: data
    pattern: 'src/repositories/**'
    allow:
      - shared
    deny:
      - presentation
      - business

  - name: shared
    pattern: 'src/utils/**'
    allow: []
    deny:
      - presentation
      - business
      - data
```

Edit the `pattern` values to match your actual directory layout.

## 5. Compile the Architecture

```bash
anvil architecture compile
```

```
Compiling architecture...
  4 layers
  4 dependency rules
  28 files mapped

Architecture compiled successfully.
```

Compilation resolves glob patterns against your file tree and builds the import
graph used during validation.

## 6. Validate

```bash
anvil check --all
```

```
Checking architecture...
  ARCH-001: Boundary violation
    src/api/handlers/report.ts:3
    imports from ../../repositories/report.repo
    Rule: presentation denies imports from data

1 architecture violation found.
```

## 7. Fix or Suppress

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

:::caution Suppressions require a reason. A bare `@anvil-ignore` without an
explanation triggers its own warning. :::

---

**Next:** [Drift Detection](/anvil/tutorials/drift)
