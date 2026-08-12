# Custom Architecture Policies Guide

| Type  | Authority | Owner   | Status | Freshness                                                                                                                                       |
| ----- | --------- | ------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Advisory  | ARCHCFG | Live   | Last reviewed 2026-08-13 against `anvil architecture --help`, `crates/anvil-cli/src/commands/architecture.rs`, and the UCFG-008 resolution seam |

| Upstream                                                                                                                           | Downstream                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------ |
| `plans/modules/architecture-config-validation.aps.md`, `crates/anvil-cli/src/commands/architecture.rs`, `.anvil/architecture.yaml` | Architecture-policy users, `anvil architecture`, public docs |

Define and enforce your own architectural boundaries without writing any Rego.

## Quick Start

`anvil architecture` registers only **`validate`** and **`show`** today.
Boundary enforcement, live feedback, and export use other surfaces:

```bash
# Define architecture in your project config's `architecture` section
# (inline or via `architecture.source`), or create the standalone
# .anvil/architecture.yaml (edit manually or copy a template — see below)

# Validate the resolved definition (parse + depends_on reference checks)
anvil architecture validate

# Full import-boundary enforcement
anvil gate --only-checks import-boundaries

# Save-time feedback while coding
anvil watch

# Interactive architecture-health dashboard
anvil dashboard architecture
```

Planned CLI subcommands (`init`, `check`, `visualise`, …) are tracked under
ARCHCFG-006..014 in
[`plans/modules/architecture-config-validation.aps.md`](../../plans/modules/architecture-config-validation.aps.md).

---

## Architecture Definition File

Since ADR-120 the definition's unified home is the `architecture` section of
your project config (`.anvil.yaml`), either inline or delegated to a file via
`architecture.source: ".anvil/architecture.yaml"`; the standalone
`.anvil/architecture.yaml` keeps working as the legacy form, and
`anvil migrate architecture --apply` writes the explicit source line for you.
The examples below use the standalone-file spelling — the same keys work inline
under `architecture:`. Create `.anvil/architecture.yaml` in your project root:

```yaml
schema_version: '1.0'
name: 'My Application Architecture'

layers:
  domain:
    paths: ['src/domain/**']

  application:
    paths: ['src/application/**']

  infrastructure:
    paths: ['src/infrastructure/**']

rules:
  - layer: domain
    can_import: []

  - layer: application
    can_import: [domain]

  - layer: infrastructure
    can_import: [domain, application]
```

That's it! Anvil will now enforce that:

- `domain` has no dependencies
- `application` can only import from `domain`
- `infrastructure` can import from `domain` and `application`

---

## Layer Definitions

### Basic Layer

```yaml
layers:
  domain:
    paths: ['src/domain/**']
```

### Layer with Multiple Paths

```yaml
layers:
  domain:
    paths:
      - 'src/domain/**'
      - 'src/core/**'
      - 'src/entities/**'
```

### Layer with Description

```yaml
layers:
  domain:
    paths: ['src/domain/**']
    description: 'Core business logic - must have no external dependencies'
```

### Layer with Metadata

```yaml
layers:
  domain:
    paths: ['src/domain/**']
    description: 'Core business logic'
    owner: '@core-team'
    criticality: high # high | medium | low
```

---

## Dependency Rules

### Allow Specific Dependencies

```yaml
rules:
  - layer: application
    can_import: [domain] # Only domain allowed
```

### Deny Specific Dependencies

```yaml
rules:
  - layer: presentation
    can_import: [application]
    cannot_import: [infrastructure, domain] # Explicit denials
```

### Allow All Except

```yaml
rules:
  - layer: infrastructure
    cannot_import: [presentation] # Everything else allowed
```

### Bidirectional Rules

```yaml
rules:
  # Presentation and infrastructure should never talk directly
  - between: [presentation, infrastructure]
    allowed: false
    message: 'Use application layer as intermediary'
```

---

## Module Boundaries

For larger applications, define bounded contexts or modules:

```yaml
modules:
  ordering:
    paths: ['src/modules/ordering/**']
    public_api: ['src/modules/ordering/index.ts']
    description: 'Order management'

  inventory:
    paths: ['src/modules/inventory/**']
    public_api: ['src/modules/inventory/index.ts']

  shipping:
    paths: ['src/modules/shipping/**']
    public_api: ['src/modules/shipping/index.ts']

module_rules:
  # Modules must use each other's public APIs
  - enforce: public_api_only
    severity: error
    message: 'Import from module index.ts, not internal files'

  # Specific allowed interactions
  - from: ordering
    can_import: [inventory]

  - from: shipping
    can_import: [ordering, inventory]
```

---

## File-Level Rules

Control imports at the file level:

```yaml
file_rules:
  # Test files cannot import infrastructure directly
  - pattern: '**/*.test.ts'
    cannot_import: ['src/infrastructure/**']
    message: 'Tests should use mocks, not real infrastructure'

  # React components cannot import server code
  - pattern: 'src/components/**/*.tsx'
    cannot_import:
      - 'src/server/**'
      - 'src/api/**'
      - '**/prisma/**'
    message: 'Client components cannot import server-side code'

  # Utility functions should be pure
  - pattern: 'src/utils/**'
    can_import:
      - 'src/types/**'
      - 'src/constants/**'
    cannot_import: ['src/**'] # No other src imports
    message: 'Utils should be pure and dependency-free'
```

---

## Import Restrictions

Control which packages can be used where:

```yaml
import_rules:
  # Ban packages globally
  - ban: ['lodash', 'moment', 'request']
    message: 'Use native methods, date-fns, or fetch instead'
    severity: error

  # Restrict packages to specific layers
  - package: 'prisma'
    only_in: [infrastructure]
    message: 'Database access must be in infrastructure layer'

  - package: 'express'
    only_in: [presentation, infrastructure]
    message: 'Express should not be in domain or application'

  # Warn about packages (don't block)
  - package: 'axios'
    warn_in: [domain, application]
    message: 'Consider using fetch for fewer dependencies'
    severity: warning

  # Internal package boundaries
  - package: '@internal/database'
    only_in: [infrastructure]

  - package: '@internal/ui'
    only_in: [presentation]
```

---

## Using Templates

Start from a pre-built template:

```yaml
schema_version: '1.0'
name: 'My App'
template: hexagonal # layered | hexagonal | clean | ddd

# Override or extend template layers
layers:
  # Add a new layer
  shared:
    paths: ['src/shared/**']

  # Override template layer paths
  domain:
    paths: ['src/core/**'] # Instead of template default
```

### Available Templates

#### Layered Architecture

```
template: layered

Default layers:
  presentation → business → data
```

#### Hexagonal (Ports & Adapters)

```
template: hexagonal

Default layers:
  domain (core, no deps)
  application (ports, uses domain)
  adapters (implements ports)
```

#### Clean Architecture

```
template: clean

Default layers:
  entities → use_cases → interface_adapters → frameworks
```

#### Domain-Driven Design

```
template: ddd

Supports:
  - Bounded contexts
  - Context mapping
  - Anti-corruption layers
```

---

## Severity Levels

Control how violations are handled:

```yaml
rules:
  # Blocking violation
  - layer: domain
    cannot_import: [infrastructure]
    severity: error # Blocks PR/build

  # Warning (allows PR but shows warning)
  - layer: application
    cannot_import: [presentation]
    severity: warning

  # Informational only
  - layer: shared
    cannot_import: [domain]
    severity: info
```

### Global Severity Settings

```yaml
settings:
  # Only block on errors (warnings don't fail CI)
  severity_threshold: error

  # Or be strict about warnings too
  severity_threshold: warning
```

---

## Baseline & Exceptions

### Adding Existing Violations to Baseline

Don't fix everything at once—baseline existing issues:

```yaml
baseline:
  # Ignore specific files
  - path: 'src/legacy/**'
    reason: 'Legacy code, scheduled for refactoring Q2'
    expires: '2026-06-01'

  # Ignore specific violations
  - from: 'src/api/old-handler.ts'
    to: 'src/infrastructure/db.ts'
    reason: 'Tech debt ticket: JIRA-1234'

  # Ignore entire rules temporarily
  - rule: 'layer-boundary'
    paths: ['src/migration/**']
    reason: 'Migration in progress'
    expires: '2026-02-01'
```

### Inline Exceptions

Add exceptions directly in code:

```typescript
// anvil-ignore architecture: Legacy code migration
import { db } from '../../infrastructure/database';

// anvil-ignore-next-line architecture
import { legacyHelper } from '../../../legacy/helpers';
```

---

## Validation Messages

Provide helpful messages when rules are violated:

```yaml
rules:
  - layer: domain
    cannot_import: [infrastructure]
    message: |
      Domain layer cannot depend on infrastructure.

      Instead of importing database/API clients directly:
      1. Define an interface in domain/ports/
      2. Implement it in infrastructure/adapters/
      3. Inject the implementation via dependency injection

      See: docs/architecture/dependency-rule.md
    documentation_url: 'https://wiki.acme.com/architecture/domain-layer'
```

---

## Complete Example

Here's a full architecture definition for a typical application:

```yaml
schema_version: '1.0'
name: 'E-Commerce Platform'
template: hexagonal

settings:
  severity_threshold: error
  auto_baseline: false
  strict_mode: false

layers:
  # Core business logic - absolutely no dependencies
  domain:
    paths:
      - 'src/domain/**'
      - 'src/core/**'
    description: 'Business entities, value objects, domain services'
    owner: '@core-team'
    criticality: high

  # Application services and use cases
  application:
    paths:
      - 'src/application/**'
      - 'src/use-cases/**'
    description: 'Application services, command/query handlers'

  # External integrations
  infrastructure:
    paths:
      - 'src/infrastructure/**'
      - 'src/adapters/**'
    description: 'Database, APIs, message queues, external services'

  # User interfaces
  presentation:
    paths:
      - 'src/api/**'
      - 'src/web/**'
      - 'src/cli/**'
    description: 'REST API, GraphQL, web UI, CLI'

  # Shared utilities (careful!)
  shared:
    paths: ['src/shared/**']
    description: 'Shared types, constants, pure utilities'

# Layer dependency rules
rules:
  - layer: domain
    can_import: [shared]
    cannot_import: [application, infrastructure, presentation]
    severity: error
    message: 'Domain must be pure. Use ports/adapters pattern.'

  - layer: application
    can_import: [domain, shared]
    cannot_import: [infrastructure, presentation]
    message: 'Application defines ports. Infrastructure implements them.'

  - layer: infrastructure
    can_import: [domain, application, shared]
    cannot_import: [presentation]

  - layer: presentation
    can_import: [application, shared]
    cannot_import: [domain, infrastructure]
    message: 'Presentation must go through application layer.'

  - layer: shared
    can_import: []
    message: 'Shared must be pure with no internal dependencies.'

# Bounded contexts / modules
modules:
  orders:
    paths: ['src/**/orders/**']
    public_api: ['src/modules/orders/index.ts']

  payments:
    paths: ['src/**/payments/**']
    public_api: ['src/modules/payments/index.ts']

  inventory:
    paths: ['src/**/inventory/**']
    public_api: ['src/modules/inventory/index.ts']

module_rules:
  - enforce: public_api_only
    severity: error

  - from: orders
    can_import: [payments, inventory]

  - from: payments
    can_import: [] # Payments is independent

# File-specific rules
file_rules:
  - pattern: '**/*.test.ts'
    cannot_import: ['src/infrastructure/**']

  - pattern: 'src/components/**'
    cannot_import: ['src/server/**', '**/prisma/**']

  - pattern: 'src/shared/**'
    can_import: []

# Package restrictions
import_rules:
  - ban: ['lodash', 'moment', 'request']
    message: 'Use modern alternatives'

  - package: 'prisma'
    only_in: [infrastructure]

  - package: 'express'
    only_in: [presentation, infrastructure]

  - package: 'react'
    only_in: [presentation]

# Known violations to address later
baseline:
  - path: 'src/legacy/**'
    reason: 'Legacy migration in progress'
    ticket: 'TECH-456'
    expires: '2026-06-01'
```

---

## CLI Commands

### Registered under `anvil architecture`

```bash
anvil architecture validate    # Validate the resolved definition; basic ref checks
anvil architecture show        # Print the active definition
# Both resolve the config's `architecture` section first (inline or
# source-delegated), then fall back to the standalone .anvil/architecture.yaml;
# pass --file to operate on an explicit file instead.
```

### Substitute surfaces (same capability, different command)

| Documented / planned capability | Use instead                                                                                                   |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| Initialise a definition         | Add an `architecture` section to your config, or create `.anvil/architecture.yaml` manually (templates below) |
| Check boundaries                | `anvil gate --only-checks import-boundaries`                                                                  |
| Watch boundaries continuously   | `anvil watch` (default `--action check`)                                                                      |
| Visualise the graph             | `anvil dashboard architecture` (TUI)                                                                          |
| Export architecture context     | `anvil export --format prompt-fragment` (agent context)                                                       |
| List / inspect impact           | `anvil architecture show`; gate output for violations                                                         |
| Debug configuration             | `anvil doctor`; gate verbose output                                                                           |

```bash
anvil gate --only-checks import-boundaries
anvil watch
anvil dashboard architecture
anvil export --format prompt-fragment
```

Candidate first-class subcommands remain in ARCHCFG-007..014 (blocked on
ARCHCFG-006 design gate).

---

## Escape Hatch: Custom Rego

For complex rules that YAML can't express, drop down to Rego:

```yaml
# In architecture.yaml
custom_policies:
  - name: no-circular-modules
    description: 'Prevent circular dependencies between modules'
    rego: |
      package anvil.architecture.custom

      import future.keywords.if
      import future.keywords.in

      violation contains result if {
        # Access module dependency graph
        module_a := input.architecture.modules[_]
        module_b := input.architecture.modules[_]

        # Check for cycles
        depends_on(module_a, module_b)
        depends_on(module_b, module_a)

        result := {
          "rule": "no-circular-modules",
          "severity": "error",
          "message": sprintf("Circular dependency: %s <-> %s", [
            module_a.name, module_b.name
          ])
        }
      }

      depends_on(a, b) if {
        dep := a.dependencies[_]
        dep == b.name
      }
```

Or put Rego files in `.anvil/policies/` alongside the YAML rules:

```
.anvil/
  architecture.yaml          # YAML rules
  policies/
    custom-complexity.rego   # Custom Rego rules
    team-conventions.rego
```

---

## Best Practices

### Start Small

```yaml
# Week 1: Just layers
layers:
  frontend: { paths: ['src/frontend/**'] }
  backend: { paths: ['src/backend/**'] }

rules:
  - layer: frontend
    cannot_import: [backend]
```

### Iterate

```yaml
# Week 2: Add more structure
layers:
  frontend: ...
  backend-api: { paths: ['src/backend/api/**'] }
  backend-services: { paths: ['src/backend/services/**'] }
  backend-data: { paths: ['src/backend/data/**'] }

rules:
  - layer: backend-api
    can_import: [backend-services]
  - layer: backend-services
    can_import: [backend-data]
  - layer: backend-data
    can_import: []
```

### Use Baseline Generously

Don't try to fix everything at once:

```yaml
baseline:
  # Acknowledge existing debt
  - path: 'src/legacy/**'
    reason: 'Will refactor in Q2'
    expires: '2026-06-01'
```

### Provide Good Messages

```yaml
rules:
  - layer: domain
    cannot_import: [infrastructure]
    message: |
      The domain layer must not depend on infrastructure.

      Quick fix: Define an interface and use dependency injection.

      Example:
        // In domain/ports/UserRepository.ts
        export interface UserRepository {
          findById(id: string): Promise<User>;
        }

        // In infrastructure/adapters/PrismaUserRepository.ts
        export class PrismaUserRepository implements UserRepository {
          // implementation
        }
```

---

## Troubleshooting

### "Too many violations"

Use baseline to acknowledge existing issues, then re-run the gate check:

```bash
anvil gate --only-checks import-boundaries
```

Refresh `anvil/baseline.json` through the documented baseline workflow when you
intentionally adopt new findings.

### "Rule doesn't match my files"

Check path patterns in `.anvil/architecture.yaml` and confirm files are under
the watched tree. `anvil architecture show` prints the loaded definition; gate
verbose output (`anvil gate --only-checks import-boundaries -v`) lists matched
violations.

### "False positives"

Add specific exclusions:

```yaml
rules:
  - layer: domain
    cannot_import: [infrastructure]
    exclude:
      - 'src/domain/**/*.test.ts' # Tests can import anything
```

---

## Next Steps

- [OPA Enhancement Vision](../archive/planning/opa-enhancement-vision.md) — Full
  roadmap
- Policy Library — Pre-built policies (planned)
- Natural Language Policies — Coming soon
