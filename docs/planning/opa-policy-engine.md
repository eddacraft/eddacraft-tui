# OPA Policy Engine & Architecture Validation

**Status:** In Progress **Priority:** High (Most Requested Feature) **Last
Updated:** December 2025

## Overview

This plan integrates Open Policy Agent (OPA) into Anvil's gate system to provide
flexible, industry-standard policy evaluation. Additionally, it builds an
architecture validation layer that allows organisations to define and enforce
architectural boundaries using templates and dependency analysis.

### Why OPA?

- **Industry Standard**: OPA is the CNCF-graduated policy engine used by
  Kubernetes, Terraform, and many enterprise tools
- **Flexible**: Rego policies can express complex business rules that static
  checks cannot
- **Testable**: OPA policies have first-class unit testing support
- **Composable**: Policies can be shared, versioned, and reused across
  organisations

### Why Architecture Validation?

- **Enforce Boundaries**: Prevent architectural drift by validating imports
- **Codify Patterns**: Express Clean Architecture, Hexagonal, DDD as code
- **Early Detection**: Catch boundary violations in PRs, not production
- **Team Alignment**: Architecture definitions serve as living documentation

---

## Part A: Core OPA Integration

### Phase 1: OPA Binary Management ✅ COMPLETE

**File:** `core/src/gate/policy/opa-binary-manager.ts`

Handles automatic download, caching, and version management of the OPA binary.

**Key Features:**

- Auto-downloads OPA binary on first use (~50MB per platform)
- Caches in `~/.anvil/bin/opa-{version}-{platform}-{arch}`
- Supports Linux (x64, arm64), macOS (x64, arm64), Windows (x64)
- Pins to OPA v0.60.0 by default (configurable)
- Verifies version with `opa version`

**Environment Overrides:**

| Variable            | Purpose                  |
| ------------------- | ------------------------ |
| `ANVIL_OPA_PATH`    | Use specific binary path |
| `ANVIL_OPA_VERSION` | Pin to specific version  |

**API:**

```typescript
import { OPABinaryManager, getOPABinaryManager } from '@anvil/core';

const manager = getOPABinaryManager();

// Ensure binary is available (downloads if needed)
const binaryPath = await manager.ensureBinary();

// Get binary info
const info = await manager.getBinaryInfo();
// { path: '/home/user/.anvil/bin/opa-0.60.0-linux-amd64', version: '0.60.0', platform: 'linux', arch: 'amd64' }

// Force re-download
await manager.forceDownload();
```

---

### Phase 2: Policy Check Implementation 🚧 IN PROGRESS

**Files:**

- `core/src/gate/policy/policy-loader.ts` ✅ COMPLETE
- `core/src/gate/policy/opa-executor.ts` ⏳ PENDING
- `core/src/gate/checks/policy.check.ts` ⏳ PENDING

#### Policy Loader

Discovers and loads `.rego` policy files from the workspace.

**Key Features:**

- Recursively scans `.anvil/policies/` directory
- Excludes `*_test.rego` files (test files loaded separately)
- Extracts package names from Rego source
- Supports enabled/disabled policy filters

**API:**

```typescript
import { PolicyLoader } from '@anvil/core';

const loader = new PolicyLoader();

const result = await loader.loadPolicies('/path/to/workspace', {
  policyDir: '.anvil/policies', // Optional, this is the default
  enabledPolicies: ['coverage_min'], // Optional whitelist
  disabledPolicies: ['experimental'], // Optional blacklist
});

// result.policies: LoadedPolicy[]
// result.errors: Array<{ path, error }>
// result.directory: string
```

#### OPA Executor (Planned)

Executes OPA binary and parses results.

**Planned API:**

```typescript
import { OPAExecutor } from '@anvil/core';

const executor = new OPAExecutor(binaryPath);

const result = await executor.evaluate({
  policies: loadedPolicies,
  input: {
    plan: apsDocument,
    context: { workspace_root: '/path', timestamp: Date.now() },
  },
  query: 'data.anvil.policies',
});

// result.violations: Array<{ rule, severity, message, path }>
// result.metadata: { policy_count, execution_time_ms }
```

#### Policy Check (Planned)

Main check class integrated with the gate runner.

**Planned Flow:**

1. Ensure OPA binary is available
2. Load policies from `.anvil/policies/`
3. Prepare input (APS plan → OPA input JSON)
4. Execute `opa eval --data .anvil/policies/ --input input.json`
5. Parse violations from result
6. Calculate score: `100 - (errors × 20) - (warnings × 5)`
7. Return `GateResult`

---

### Phase 3: Policy CLI Commands

**File:** `cli/src/commands/policy.ts`

| Command                 | Description                             |
| ----------------------- | --------------------------------------- |
| `anvil policy list`     | Show active policies with metadata      |
| `anvil policy validate` | Check Rego syntax for a policy file     |
| `anvil policy test`     | Run policy unit tests (`*_test.rego`)   |
| `anvil policy init`     | Create `.anvil/policies/` with examples |

**Example Usage:**

```bash
# Initialise policies directory with examples
anvil policy init

# List all active policies
anvil policy list
# Output:
# ┌─────────────────┬─────────────────────────────────┬──────────┐
# │ Policy          │ Description                     │ Tests    │
# ├─────────────────┼─────────────────────────────────┼──────────┤
# │ coverage_min    │ Enforce minimum test coverage   │ 3 tests  │
# │ change_scope    │ Limit files per plan            │ 2 tests  │
# │ security_base   │ Security review requirements    │ 4 tests  │
# └─────────────────┴─────────────────────────────────┴──────────┘

# Validate a specific policy
anvil policy validate .anvil/policies/my-policy.rego

# Run all policy tests
anvil policy test

# Run tests for specific policy
anvil policy test coverage_min
```

---

### Phase 4: Example Policies

**Location:** `core/src/gate/__fixtures__/policies/`

These are copied to `.anvil/policies/` when running `anvil policy init`.

#### 1. Coverage Minimum (`coverage_min.rego`)

```rego
package anvil.policies.coverage_min

import future.keywords.if
import future.keywords.in

default allow := true

# Minimum coverage threshold (configurable via input.config)
min_coverage := object.get(input, ["config", "min_coverage"], 80)

violation[msg] {
  coverage := input.context.coverage.lines
  coverage < min_coverage
  msg := sprintf("Test coverage %v%% is below minimum %v%%", [coverage, min_coverage])
}
```

#### 2. Change Scope (`change_scope.rego`)

```rego
package anvil.policies.change_scope

import future.keywords.if
import future.keywords.in

# Maximum files per plan
max_files := object.get(input, ["config", "max_files"], 20)

# Maximum directories per plan
max_directories := object.get(input, ["config", "max_directories"], 5)

violation[msg] {
  count(input.plan.proposed_changes) > max_files
  msg := sprintf("Plan touches %v files, maximum is %v", [
    count(input.plan.proposed_changes), max_files
  ])
}

violation[msg] {
  directories := {dir | change := input.plan.proposed_changes[_]; dir := change.directory}
  count(directories) > max_directories
  msg := sprintf("Plan touches %v directories, maximum is %v", [
    count(directories), max_directories
  ])
}
```

#### 3. Security Baseline (`security_baseline.rego`)

```rego
package anvil.policies.security_baseline

import future.keywords.if
import future.keywords.in

# Paths requiring security review
sensitive_paths := [
  "src/auth/**",
  "src/security/**",
  "**/credentials*",
  "**/secrets*",
  "*.env*"
]

violation[msg] {
  change := input.plan.proposed_changes[_]
  is_sensitive(change.path)
  not has_security_review(input.plan)
  msg := sprintf("Changes to %s require security review tag", [change.path])
}

is_sensitive(path) {
  pattern := sensitive_paths[_]
  glob.match(pattern, ["/"], path)
}

has_security_review(plan) {
  "security-review" in plan.tags
}
```

---

## Part B: Architecture Validation

### Phase 5: Architecture Definition System

**Files:**

- `core/src/architecture/types.ts` - TypeScript types
- `core/src/architecture/parser.ts` - Parse YAML definitions
- `core/src/architecture/validator.ts` - Validate against architecture

#### Architecture Definition Format

**File:** `.anvil/architecture.yaml`

```yaml
schema_version: '1.0'
name: 'MyApp Architecture'

# Choose a base template or define custom
template: hexagonal # layered | hexagonal | clean | ddd | custom

# Layer definitions (override or extend template)
layers:
  domain:
    paths: ['src/domain/**', 'src/core/**']
    description: 'Business logic and entities'

  application:
    paths: ['src/application/**', 'src/use-cases/**']
    description: 'Application services and use cases'
    depends_on: [domain]

  infrastructure:
    paths: ['src/infrastructure/**', 'src/adapters/**']
    description: 'External integrations, DB, APIs'
    depends_on: [domain, application]

  presentation:
    paths: ['src/api/**', 'src/web/**', 'src/cli/**']
    description: 'User-facing interfaces'
    depends_on: [application]

# Explicit dependency rules
rules:
  - from: domain
    allow: [] # Domain has no external dependencies

  - from: application
    allow: [domain]

  - from: infrastructure
    allow: [domain, application]
    deny_except: ['src/infrastructure/shared/**']

  - from: presentation
    allow: [application]
    deny: [infrastructure] # No direct infra access

# Escape hatch to full Rego
custom_policies:
  - name: no_circular_deps
    rego: |
      package anvil.architecture.custom
      violation[msg] {
        # Custom Rego logic here
      }
```

---

### Phase 6: Architecture Templates

Pre-built templates in `core/src/architecture/templates/`:

#### Layered Architecture (`layered.yaml`)

Traditional three-tier architecture.

```
┌─────────────────────────────┐
│       Presentation          │  → UI, Views, Controllers
├─────────────────────────────┤
│         Business            │  → Services, Domain Logic
├─────────────────────────────┤
│           Data              │  → Repositories, DAOs
└─────────────────────────────┘
```

#### Hexagonal / Ports & Adapters (`hexagonal.yaml`)

Isolates business logic from external concerns.

```
           ┌───────────────────┐
           │    Application    │
           └────────┬──────────┘
                    │
        ┌───────────┼───────────┐
        │           │           │
   ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
   │  Port   │ │  Port   │ │  Port   │
   └────┬────┘ └────┬────┘ └────┬────┘
   ┌────┴────┐ ┌────┴────┐ ┌────┴────┐
   │ Adapter │ │ Adapter │ │ Adapter │
   └─────────┘ └─────────┘ └─────────┘
        │           │           │
      REST        DB        Message
```

#### Clean Architecture (`clean.yaml`)

Dependency rule: outer layers depend on inner layers.

```
┌─────────────────────────────────────────┐
│             Frameworks & Drivers        │
│  ┌─────────────────────────────────┐    │
│  │       Interface Adapters        │    │
│  │  ┌─────────────────────────┐    │    │
│  │  │       Use Cases         │    │    │
│  │  │  ┌─────────────────┐    │    │    │
│  │  │  │    Entities     │    │    │    │
│  │  │  └─────────────────┘    │    │    │
│  │  └─────────────────────────┘    │    │
│  └─────────────────────────────────┘    │
└─────────────────────────────────────────┘
```

#### Domain-Driven Design (`ddd.yaml`)

Supports bounded contexts and context mapping.

```yaml
bounded_contexts:
  - name: ordering
    paths: ['src/**/ordering/**']
  - name: inventory
    paths: ['src/**/inventory/**']
  - name: shipping
    paths: ['src/**/shipping/**']

context_rules:
  - between: [ordering, inventory]
    via: ['src/interfaces/inventory-api/**']
```

---

### Phase 7: Dependency-Cruiser Integration

**File:** `core/src/architecture/dependency-cruiser-adapter.ts`

Integrates [dependency-cruiser](https://github.com/sverweij/dependency-cruiser)
for static dependency analysis.

**Approach:**

1. Generate dependency-cruiser config from `.anvil/architecture.yaml`
2. Run dependency-cruiser programmatically
3. Map violations back to architecture layers
4. Return as `GateResult`

**Example Integration:**

```typescript
import { cruise } from 'dependency-cruiser';

export class ArchitectureCheck extends BaseCheck {
  name = 'architecture';

  async run(context: CheckContext): Promise<GateResult> {
    // Load architecture definition
    const archDef = await this.loadArchitecture(context.workspace_root);

    // Generate dependency-cruiser rules
    const dcConfig = this.generateDCConfig(archDef);

    // Get affected files from plan
    const files = this.getAffectedFiles(context.plan);

    // Run analysis
    const result = await cruise(files, dcConfig);

    // Map to violations
    return this.buildResult(result, archDef);
  }
}
```

---

### Phase 8: Architecture CLI Commands

**File:** `cli/src/commands/architecture.ts`

| Command                        | Description                        |
| ------------------------------ | ---------------------------------- |
| `anvil architecture init`      | Create architecture definition     |
| `anvil architecture validate`  | Validate definition syntax         |
| `anvil architecture check`     | Run architecture check standalone  |
| `anvil architecture visualise` | Generate dependency graph (future) |

**Example Usage:**

```bash
# Initialise with hexagonal template
anvil architecture init --template hexagonal

# Validate architecture definition
anvil architecture validate

# Run architecture check
anvil architecture check
# Output:
# Architecture Check Results
# ─────────────────────────────
# ✗ src/presentation/api/handler.ts
#   └─ Imports from infrastructure layer (not allowed)
#   └─ Violation: presentation → infrastructure
#
# ✓ src/application/services/order.ts
#   └─ Clean: only imports from domain layer
```

---

### Phase 9: Architecture Policies (Auto-Generated Rego)

Auto-generates Rego policies from architecture definition for use with OPA.

**Generated Output:** `.anvil/policies/.generated/architecture_boundaries.rego`

```rego
# AUTO-GENERATED - Do not edit manually
# Source: .anvil/architecture.yaml
package anvil.policies.architecture

import future.keywords.if
import future.keywords.in

# Layer definitions (from architecture.yaml)
layer_paths := {
  "domain": ["src/domain/**"],
  "application": ["src/application/**"],
  "infrastructure": ["src/infrastructure/**"]
}

# Allowed dependencies (from architecture.yaml)
allowed_deps := {
  "domain": [],
  "application": ["domain"],
  "infrastructure": ["domain", "application"]
}

# Violation detection
violation[violation] {
  change := input.plan.proposed_changes[_]
  change.type in ["file_create", "file_update"]

  from_layer := get_layer(change.path)
  imports := input.architecture.dependencies[change.path]

  import_path := imports[_]
  to_layer := get_layer(import_path)

  not to_layer in allowed_deps[from_layer]

  violation := {
    "rule": "architecture_boundary",
    "severity": "error",
    "message": sprintf("%s (%s) cannot import from %s (%s)", [
      change.path, from_layer, import_path, to_layer
    ])
  }
}
```

---

## Configuration

### `.anvilrc` Example

```json
{
  "version": 1,
  "checks": [
    { "name": "eslint", "enabled": true },
    { "name": "coverage", "enabled": true },
    {
      "name": "policy",
      "enabled": true,
      "config": {
        "policy_dir": ".anvil/policies",
        "severity_threshold": "error"
      }
    },
    {
      "name": "architecture",
      "enabled": true,
      "config": {
        "definition": ".anvil/architecture.yaml",
        "fail_on_violation": true
      }
    }
  ],
  "thresholds": {
    "overall_score": 80
  }
}
```

---

## Implementation Status

| Phase | Description                    | Status         |
| ----- | ------------------------------ | -------------- |
| 1     | OPA Binary Management          | ✅ Complete    |
| 2     | Policy Check Implementation    | 🚧 In Progress |
| 3     | Policy CLI Commands            | ⏳ Pending     |
| 4     | Example Policies               | ⏳ Pending     |
| 5     | Architecture Definition System | ⏳ Pending     |
| 6     | Architecture Templates         | ⏳ Pending     |
| 7     | Dependency-Cruiser Integration | ⏳ Pending     |
| 8     | Architecture CLI Commands      | ⏳ Pending     |
| 9     | Architecture Policies          | ⏳ Pending     |

---

## Dependencies

**npm packages:**

- `dependency-cruiser` (MIT) - Architecture validation

**External binaries:**

- OPA v0.60.0 (auto-downloaded)

---

## Future: Remote Policy Bundles

Design for centralised policy distribution (not in initial scope):

```yaml
# .anvilrc policy configuration
policy:
  bundles:
    - name: org-standards
      url: https://policies.myorg.com/anvil/bundle.tar.gz
      refresh_interval: 60m
      signature_key: /path/to/public.key
  local_dir: .anvil/policies
  cache_dir: .anvil/policy-cache
```

---

## Related Documents

- [TODO.md](./TODO.md) - Task tracking
- [ROADMAP.md](./ROADMAP.md) - Strategic roadmap
- [ARCHITECTURE.md](../ARCHITECTURE.md) - System design
