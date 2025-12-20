# Anvil Feature Architecture Plan

## Overview

This document describes **how** Anvil's major features integrate as a coherent
system. For **what** features and **when** (status, priorities), see
[ROADMAP.md](./ROADMAP.md).

**Features covered**:

- Speed & Caching (Priority 1)
- CI/CD Integration (Priority 2)
- Architecture Validation (Priority 4)
- Apply/Rollback (Priority 6)
- VS Code/Cursor Extension (Priority 5)
- MCP Server (Priority 5)

**Business Context**: Free tier (not OSS), paid tiers. Primary users:
professional AI-heavy developers → enterprise teams → vibe coders.

---

## Core Abstractions (Shared Infrastructure)

All features share these abstractions for coherent integration:

### 1. CacheProvider (`core/src/cache/`)

```typescript
interface CacheProvider {
  get<T>(key: string): Promise<CacheEntry<T> | null>;
  set<T>(key: string, value: T, options?: CacheSetOptions): Promise<void>;
  invalidate(key: string): Promise<boolean>;
  invalidatePattern(pattern: string): Promise<number>;
  getStats(): Promise<CacheStats>;
}
```

**Implementations**: FileCacheProvider (default), MemoryCacheProvider (watch
mode), NullCacheProvider (--no-cache)

### 2. OutputFormatter (`core/src/output/`)

```typescript
interface OutputFormatter {
  format: 'human' | 'json' | 'github' | 'sarif';
  formatGateResults(results: GateRunResult): string;
  formatValidationErrors(errors: ValidationError[]): string;
}
```

**Implementations**: HumanFormatter (current), JSONFormatter (CI),
GitHubFormatter (annotations), SARIFFormatter (security)

### 3. ExecutionEngine (`core/src/execution/`)

```typescript
interface ExecutionEngine {
  dryRun(plan: APSPlan): Promise<DryRunResult>;
  apply(plan: APSPlan): Promise<ApplyResult>;
  rollback(snapshotId: string): Promise<RollbackResult>;
}
```

### 4. AnvilContext (Runtime Context)

Shared context that feeds into OPA for progressive policy integration:

```typescript
interface AnvilContext {
  workspaceRoot: string;
  config: AnvilConfig;
  cache: CacheProvider;
  formatter: OutputFormatter;
  ciContext?: CIContext;

  // OPA-accessible data
  toOPAInput(): {
    cache: { enabled; hit_rate; last_validation_cached };
    ci: { provider; pr_number; branch; author };
    execution_history: { last_apply; rollback_count };
  };
}
```

---

## Feature Dependencies

```
┌─────────────────────────────────────┐
│         Core Abstractions           │
│  (Cache, Output, Execution, Context)│
└───────────────┬─────────────────────┘
                │
    ┌───────────┼───────────┐
    │           │           │
    ▼           ▼           ▼
┌────────┐  ┌────────┐  ┌────────────┐
│ Speed  │  │ CI/CD  │  │Architecture│
│ Cache  │  │ Output │  │ Validation │
└───┬────┘  └───┬────┘  └─────┬──────┘
    │           │             │
    └─────┬─────┴─────────────┘
          │
          ▼
    ┌──────────────┐
    │GitHub Action │
    └──────┬───────┘
           │
   ┌───────┼───────┐
   │       │       │
   ▼       ▼       ▼
┌──────┐ ┌────┐ ┌────────┐
│VSCode│ │MCP │ │ Apply/ │
│ Ext  │ │Srv │ │Rollback│
└──────┘ └────┘ └────────┘
```

---

## Tier Boundaries

### Free Tier

- All core validation and gate checks
- Local caching (`.anvil/cache/`)
- JSON output format
- Watch mode
- Architecture validation (layered template only)
- Apply/rollback (local only)
- VS Code extension (basic)
- MCP server (validation tools)

### Team Tier ($29/dev/month)

- GitHub Action with PR integration
- All architecture templates (hexagonal, clean, DDD)
- Custom architecture definitions
- Shared team policies (cloud sync)
- Extended cache (shared across team)

### Enterprise Tier (Custom)

- SSO/SAML, RBAC
- Compliance reporting
- Audit dashboard
- On-premise option

---

## Implementation Phases

### Phase 1: Foundation (Core Abstractions)

**Files to create:**

```
core/src/cache/
├── types.ts
├── providers/file-cache.ts
├── providers/memory-cache.ts
├── providers/null-cache.ts
└── cache-key.ts

core/src/output/
├── types.ts
├── formatters/human.ts (extract from cli/utils/output.ts)
├── formatters/json.ts
└── formatters/github.ts
```

**Key changes:**

- Extract `cli/src/utils/output.ts` formatters into core
- Create cache key strategy using plan hash + check config hash
- Add `--output` and `--no-cache` flags to gate command

---

### Phase 2: Speed & Caching

**Modify:** `core/src/gate/gate-runner.ts`

```typescript
// Change from sequential:
for (const checkConfig of config.checks) {
  const result = await check.run(context);
}

// To parallel:
const results = await Promise.all(
  checks.map((check) => this.runCheckWithCache(check, context))
);
```

**Add watch mode:** `cli/src/commands/watch.ts`

- Use chokidar for file watching
- MemoryCacheProvider for fast re-validation
- Debounce rapid changes (300ms default)
- `--gate` flag to run full gates vs just validation

**CLI changes:**

```bash
anvil gate <plan> --no-cache          # Bypass cache
anvil gate <plan> --parallel=4        # Limit parallelism
anvil watch .                         # Watch mode
anvil watch . --gate                  # Watch with gates
```

---

### Phase 3: CI/CD Integration

**Files to create:**

```
.github/actions/anvil-gate/
├── action.yml
└── README.md

core/src/output/formatters/
├── json.ts       # Machine-readable output
└── github.ts     # Annotations format
```

**JSON output structure:**

```typescript
interface JSONGateOutput {
  version: '1.0.0';
  timestamp: string;
  overall: boolean;
  score: number;
  checks: Array<{
    name: string;
    passed: boolean;
    annotations?: Array<{
      file: string;
      line: number;
      severity: 'error' | 'warning' | 'notice';
      message: string;
    }>;
  }>;
  cache?: { hit: boolean };
  ci?: { provider: string; prNumber?: string };
}
```

**GitHub Action features:**

- Auto-install Anvil CLI
- Run gate with CI profile
- Post PR summary comment
- Create file annotations for ESLint/secret findings
- Set commit status

---

### Phase 4: Architecture Validation (OPA Phases 5-9)

**Files to create:**

```
core/src/architecture/
├── types.ts
├── definition-loader.ts
├── templates/
│   ├── layered.ts
│   ├── hexagonal.ts
│   ├── clean.ts
│   └── ddd.ts
├── validators/
│   ├── dependency-validator.ts
│   └── layer-validator.ts
├── integrations/dependency-cruiser.ts
├── rego-generator.ts
└── architecture.check.ts

cli/src/commands/architecture.ts
```

**Architecture definition format:** `.anvil/architecture.yaml`

```yaml
version: '1.0'
template: hexagonal # layered | hexagonal | clean | ddd | custom

layers:
  domain:
    paths: ['src/domain/**']
    can_depend_on: []
  application:
    paths: ['src/application/**']
    can_depend_on: [domain]
  infrastructure:
    paths: ['src/infrastructure/**']
    can_depend_on: [domain, application]

rules:
  no_circular: true
  max_depth: 3
```

**CLI commands:**

```bash
anvil architecture init --template hexagonal
anvil architecture validate
anvil architecture check
anvil architecture generate-policy  # Generate Rego from YAML
```

**Auto-generated Rego** (`.anvil/policies/.generated/architecture.rego`):

- Converts layer rules to violation rules
- Integrates with existing OPA policy check
- Regenerated on `architecture.yaml` changes

---

### Phase 5: Apply/Rollback

**Files to create:**

```
core/src/execution/
├── types.ts
├── apply-engine.ts
├── rollback-engine.ts
└── file-operations.ts

core/src/snapshot/
├── types.ts
├── snapshot-manager.ts
├── snapshot-writer.ts
└── snapshot-reader.ts
```

**Snapshot structure:**

```
.anvil/snapshots/
├── index.json
└── aps-{planId}/
    ├── metadata.json
    ├── manifest.json
    └── files/
        └── {escaped-paths}
```

**Apply flow:**

1. Pre-flight checks (gate passed? architecture violations?)
2. Create snapshot of affected files
3. Apply changes transactionally (sequential, with rollback on failure)
4. Generate execution evidence
5. Return snapshotId for potential rollback

**CLI commands:**

```bash
anvil apply <plan> --dry-run         # Preview changes
anvil apply <plan>                    # Apply with snapshot
anvil apply <plan> --force            # Skip pre-flight checks
anvil rollback <snapshot-id>          # Restore from snapshot
anvil rollback <snapshot-id> --verify # Verify before restore
```

---

### Phase 6: VS Code/Cursor Extension

**Repository:** `extensions/vscode/`

**Features:**

1. **Diagnostics Provider** - Validation errors in Problems panel
2. **CodeLens** - "Validate" / "Gate" / "Apply" links on plan files
3. **Status Bar** - Gate status indicator
4. **Gate Results Panel** - Webview with detailed results
5. **Commands** - anvil.validate, anvil.gate, anvil.apply

**Architecture:**

- Wraps CLI for actual execution
- Uses JSON output mode for parsing results
- File watchers for real-time validation
- Workspace configuration for settings

---

### Phase 7: MCP Server

**Package:** `packages/mcp-server/`

**Tools exposed:**

- `anvil_validate` - Validate a plan file or content
- `anvil_gate` - Run quality gates
- `anvil_apply` - Apply plan (dry-run by default)
- `anvil_search_plans` - Find plans in workspace

**Resources exposed:**

- `anvil://gate-results/latest` - Most recent gate results
- `anvil://plans` - List of plans in workspace
- `anvil://architecture` - Architecture definition

**Key difference from VS Code:**

- Stateless per request (no file watchers)
- Structured JSON responses for AI consumption
- Tool calls vs visual UI
- Integration with Claude, Cursor, etc.

---

## Progressive OPA Integration

All features feed context into OPA for policy evaluation:

```typescript
const opaInput = {
  plan: { /* existing */ },
  context: { /* existing */ },

  // NEW: From caching
  cache: {
    enabled: true,
    hit: cachedResult !== null,
    staleness_seconds: 0,
  },

  // NEW: From CI
  ci: {
    provider: 'github',
    pr_number: '123',
    branch: 'feature/foo',
    author: 'dev@example.com',
  },

  // NEW: From apply/rollback
  execution: {
    last_apply: '2025-12-10T10:00:00Z',
    snapshot_exists: true,
    rollback_available: true,
  },

  // NEW: From architecture
  architecture: {
    template: 'hexagonal',
    violations: [...],
    layers_touched: ['application', 'domain'],
  },
};
```

**Example policies using extended context:**

```rego
# Require review for multi-layer changes
violation[msg] {
  count(input.architecture.layers_touched) > 3
  not "architecture-review" in input.plan.tags
  msg := "Changes touch >3 layers - requires architecture-review tag"
}

# Block apply on architecture violations
deny[msg] {
  count(input.architecture.violations) > 0
  msg := sprintf("Cannot apply: %d architecture violations",
    [count(input.architecture.violations)])
}

# Warn on cache miss in CI
warning[msg] {
  input.ci.provider != ""
  input.cache.hit == false
  msg := "Cache miss in CI - consider warming cache"
}
```

---

## Implementation Order

| Phase | Features                      | Parallel? | Dependencies       |
| ----- | ----------------------------- | --------- | ------------------ |
| 1     | Core Abstractions             | -         | None               |
| 2a    | Speed & Caching               | Yes       | Phase 1            |
| 2b    | CI/CD JSON Output             | Yes       | Phase 1            |
| 2c    | Architecture Definition       | Yes       | Phase 1            |
| 3     | GitHub Action                 | No        | Phase 2a, 2b       |
| 4     | Architecture Check + Rego Gen | No        | Phase 2c           |
| 5     | Apply/Rollback                | No        | Phase 4 (optional) |
| 6     | VS Code Extension             | Yes       | Phase 2b           |
| 7     | MCP Server                    | Yes       | Phase 2b           |

**Phases 2a, 2b, 2c can be built in parallel** after core abstractions. **Phases
6, 7 can be built in parallel** after JSON output.

---

## Critical Files Summary

**Modify:**

- `core/src/gate/gate-runner.ts` - Add caching + parallel execution
- `core/src/gate/policy/opa-executor.ts` - Extend OPAInput with new context
- `cli/src/commands/gate.ts` - Add --output, --no-cache, --parallel flags
- `cli/src/utils/output.ts` - Extract into OutputFormatter abstraction

**Create:**

- `core/src/cache/` - Cache infrastructure
- `core/src/output/` - Output formatters
- `core/src/execution/` - Apply/rollback engines
- `core/src/snapshot/` - Snapshot management
- `core/src/architecture/` - Architecture validation
- `cli/src/commands/watch.ts` - Watch mode
- `cli/src/commands/architecture.ts` - Architecture CLI
- `.github/actions/anvil-gate/` - GitHub Action
- `extensions/vscode/` - VS Code extension
- `packages/mcp-server/` - MCP server

---

## Storage Structure

```
.anvil/
├── config.json           # User configuration
├── cache/
│   ├── index.json        # Cache registry
│   └── entries/          # Cache entries by hash
├── snapshots/
│   ├── index.json        # Snapshot registry
│   └── {plan-id}/        # Snapshot contents
├── evidence/
│   └── {plan-id}/        # Gate evidence history
├── policies/
│   ├── *.rego            # User policies
│   └── .generated/       # Auto-generated (architecture)
├── architecture.yaml     # Architecture definition
└── state.json            # Runtime state
```

---

_Last updated: December 2025_
