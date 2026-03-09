# CLAUDE.md

> **📖 Primary Reference**: See [README.md](./README.md) for the single source
> of truth on:
>
> - Project overview and architecture
> - Development workflows and commands
> - Testing strategies and patterns
> - Configuration details
> - Contributing guidelines

This file contains **Claude Code specific** guidance and shortcuts.

## 🚀 Quick Start for Claude

### First Time in This Repo

```
/prime-repo
```

This scans the entire codebase and provides comprehensive context about:

- Architecture and design patterns
- Current implementation status
- Key files and their purposes
- Development conventions

### Common Workflows

```bash
# Add a new feature
/feature "implement BMAD adapter"

# Review and prepare for release
/ship "review CLI integration PR"

# Generate documentation
/docs-writer "update adapter README"

# Debug an issue
Use skill: .claude/skills/debug-adapter.md
Use skill: .claude/skills/fix-build.md

# Create a PRD for planning
/full-spec "Add policy engine with OPA/Rego"
```

## 🤖 Available Agents & Commands

### Agents (in `.claude/agents/`)

- **planner** – Break down goals into actionable steps
- **architect** – Design interfaces and file structures
- **product-manager** – Write PRDs with use cases
- **coder** – Implement changes with minimal diffs
- **tester** – Generate test plans and test code
- **reviewer** – Code review with pragmatic feedback
- **docs-writer** – Update documentation
- **security-auditor** – Security and compliance checks

### Slash Commands (in `.claude/commands/`)

Core Commands:

- `/new-project` – Scaffold new features/modules
- `/feature` – End-to-end feature implementation
- `/full-spec` – Detailed PRD and API spec
- `/ship` – Review, audit, and document for release
- `/demo` – Build minimal demo with UI

Git Workflow (via addon):

- `/commit` – Standardized commit messages
- `/create-pr` – Create pull requests
- `/changelog` – Generate changelogs

Repository (via addon):

- `/prime-repo` – Deep repository analysis
- `/prime-docs` – Documentation scanning

### Skills (in `.claude/skills/`)

Practical troubleshooting guides:

- `debug-adapter.md` – Debug format adapter issues
- `feature-adapter.md` – Add new adapters (SpecKit, BMAD, etc.)
- `fix-build.md` – Resolve TypeScript/build errors
- `implement-pattern.md` – Implement design patterns
- `refactor-safe.md` – Safe refactoring workflows
- `trace-data-flow.md` – Understand data transformations

**Usage**: Reference skills in your prompts:

```
Using .claude/skills/debug-adapter.md, help me debug the SpecKit parser issue
```

## 📋 Doc Templates (in `.claude/docs-templates/`)

Available templates:

- **PRD.md** – Product Requirements Document
- **ADR.md** – Architecture Decision Record
- **Architecture-Design.md** – Technical blueprints
- **Test-Plan.md** – Comprehensive test strategies
- **Security-Audit.md** – Security review checklist
- **Runbook.md** – Operational guides

The `docs-writer` agent can automatically fill these in.

## 📦 Anvil-Specific Context

### The Big Picture

Anvil is a **deterministic development automation platform**. The key
architectural insight:

```
User Formats (SpecKit, BMAD) → Adapters → APS (internal) → Gates → Execution
```

**APS (Anvil Plan Specification)** is the moat:

- Hash-stable (SHA-256) for deterministic validation
- Schema-validated (Zod) for type safety
- Users never see it – we work with their existing formats

### Current Implementation Status (October 2025)

✅ **Complete**:

- APS Core (schema, validation, hashing) – 100%
- Adapter Framework (base types, registry) – 100%
- SpecKit Adapter (v1 + v2, 51 tests) – 100%
- Gate v1 (lint, test, coverage, secrets) – 100%

🚧 **In Progress**:

- CLI Integration (format detection, multi-format support) – 80%

📋 **Planned**:

- BMAD Adapter (PRD/architecture docs)
- Policy Engine (OPA/Rego)
- Apply/Rollback with snapshots
- GitHub Action integration

### Key Design Principles

1. **Interoperability First** – Support existing formats, don't force adoption
2. **Determinism** – Same input → same output, always
3. **Safety by Default** – Snapshot before apply, rollback is first-class
4. **Transparency** – Immutable evidence bundles for audit trails
5. **Composability** – Parse → Validate → Execute are independent stages

## 🛠️ Development Commands

See [README.md](./README.md#-available-scripts) for complete command reference.

### Quick Reference

```bash
# Install & build
pnpm install && pnpm build

# Test & quality checks
pnpm test                    # Unit tests
pnpm test:coverage           # With coverage
pnpm typecheck               # Type checking
pnpm lint                    # Lint & fix

# Nx commands
npx nx build core            # Build specific package
npx nx test adapters         # Test specific package
npx nx graph                 # Visualise dependencies

# Package-specific
pnpm -F core run generate:schema          # Regenerate APS JSON schema
pnpm -F core run update-golden-hashes     # Update golden test hashes
```

### Critical: Build Before Testing

TypeScript project references require builds before cross-package imports work:

```bash
pnpm build    # Always build first
pnpm test     # Then test
```

## 🎯 Common Claude Workflows

### Adding a New Adapter

1. Start with planning:

   ```
   /feature "add BMAD adapter for PRD documents"
   ```

2. Reference the skill:

   ```
   Using .claude/skills/feature-adapter.md, help me implement the BMAD adapter
   ```

3. Follow SpecKit as reference:
   - Study `packages/adapters/src/speckit/import-adapter-v2.ts`
   - Implement `FormatAdapter` interface from `base/types.ts`
   - Register with `AdapterRegistry`
   - Add comprehensive tests (see `__tests__/speckit-import-v2.test.ts`)

### Debugging Build Issues

```
Using .claude/skills/fix-build.md, help me resolve the TypeScript error in packages/adapters
```

Common issues:

- Missing `.js` extensions in imports (ESM requirement)
- Need to run `pnpm build` before testing
- TypeScript project references not configured
- `rootDir` mismatch in `tsconfig.spec.json`

### Implementing a Gate Check

1. Plan the check:

   ```
   /architect "design OPA policy check for gate runner"
   ```

2. Implement `Check` interface:
   - See `core/src/gate/check.interface.ts`
   - Reference existing checks: `checks/eslint.check.ts`,
     `checks/coverage.check.ts`
   - Return `GateResult` with `{ check, passed, message, score?, details? }`

3. Register in `gate-runner.ts`:
   - Add to `registerDefaultChecks()`

### Reviewing Code

```
/ship "review adapter integration PR"
```

This will:

- Run the reviewer agent
- Check code quality, tests, documentation
- Generate review comments
- Suggest improvements

## 🧠 Anvil Mental Models

### The Adapter Pattern

**Problem**: Users have existing planning formats (SpecKit, BMAD, ADRs)  
**Solution**: Adapters parse external formats → APS → validate → serialize back

```typescript
// Adapter flow
ExternalFormat → adapter.parse() → APS
APS → validation → gates → evidence
APS → adapter.serialize() → ExternalFormat (with evidence)
```

**Key files**:

- `packages/adapters/src/base/types.ts` – FormatAdapter interface
- `packages/adapters/src/base/registry.ts` – Auto-detection registry
- `packages/adapters/src/speckit/import-adapter-v2.ts` – Complete example

### The Gate System

**Purpose**: Validate plans before execution (lint, test, coverage, secrets,
policies)

```typescript
// Gate flow
Plan → GateRunner.runGate() → [Check, Check, Check] → GateResult
```

**Key files**:

- `core/src/gate/gate-runner.ts` – Orchestrates checks
- `core/src/gate/check.interface.ts` – Check contract
- `core/src/gate/checks/*.check.ts` – Individual checks

### The APS Schema

**Purpose**: Internal, deterministic representation of plans

```typescript
// APS structure
{
  schema_version: "0.1.0",
  intent: { description, objectives, constraints },
  proposed_changes: [{ type, path, description, content }],
  provenance: { timestamp, author, source },
  validation: { required_checks, policy_version },
  evidence: [{ check, status, timestamp, details }]
}
```

**Key files**:

- `core/src/schema/aps.schema.ts` – Zod schema definition
- `core/src/validation/aps-validator.ts` – Validation logic
- `core/src/crypto/hash.ts` – Deterministic hashing

## 🚨 Important Conventions

### Language

**Always use UK English** for:

- Documentation and comments
- Variable names (`colour` not `color`)
- Function names (`initialise` not `initialize`)
- User-facing text

### TypeScript

**ES Modules** – All imports require `.js` extensions:

```typescript
// ✅ Correct
import { foo } from './utils.js';

// ❌ Wrong
import { foo } from './utils';
```

**Zod-First Schemas** – Define with Zod, export TypeScript types:

```typescript
export const MySchema = z.object({
  field: z.string(),
});
export type MyType = z.infer<typeof MySchema>;
```

### Testing

- Co-locate tests with source (`.test.ts` or `.spec.ts`)
- Use `__fixtures__/` for test data
- Golden files in `core/src/__fixtures__/golden-plans/`
- Update golden hashes after schema changes:
  ```bash
  pnpm -F core run update-golden-hashes
  ```

## 📖 Key Documentation

**Architecture & Design**:

- [ARCHITECTURE.md](./ARCHITECTURE.md) – System design (1,575 lines)
- [PLAN.md](./PLAN.md) – Strategic vision (three acts)
- [docs/adr/](./docs/adr/) – Architecture decisions

**Package Documentation**:

- [core/API.md](./core/API.md) – APS Core API
- [packages/adapters/README.md](./packages/adapters/README.md) – Adapter
  framework
- [packages/adapters/ADAPTER_WORKFLOW_GUIDE.md](./packages/adapters/ADAPTER_WORKFLOW_GUIDE.md)
  – Adding adapters

**Development**:

- [README.md](./README.md) – Single source of truth
- [TODO.md](./TODO.md) – Task tracking
- [.claude/USAGE.md](./.claude/USAGE.md) – Claude Projects Lite guide

## 💡 Pro Tips

1. **Always run `/prime-repo` first** when starting work on this codebase
2. **Reference skills** for common patterns:
   `Using .claude/skills/debug-adapter.md...`
3. **Use SpecKit adapter as reference** when building new adapters
4. **Check TODO.md** for current sprint focus and what's in progress
5. **Build before testing** to avoid import errors
6. **Update golden hashes** after APS schema changes

---

**For complete development workflows, commands, and contributing guidelines, see
[README.md](./README.md)**
