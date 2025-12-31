# Core Package (@anvil/core)

> APS schema, validation, gate runner, crypto, architecture analysis,
> anti-pattern detection

**Parent**: See root `AGENTS.md` for project-wide conventions.

## Structure

```
core/src/
├── schema/           # Zod schemas (source of truth for APS)
├── crypto/           # SHA-256 deterministic hashing
├── validation/       # APSValidator with rich error messages
├── gate/             # Quality gate runner + checks
│   ├── checks/       # Individual gate checks (7 checks)
│   └── policy/       # OPA/Rego policy engine
├── architecture/     # Dependency analysis, baseline management
├── antipattern/      # Pattern catalogue + scanner
├── provenance/       # Audit trail, environment, git context
├── suppression/      # Warning suppression with justification
├── cache/            # Plan caching with providers
└── watch/            # File watching utilities
```

## Where to Look

| Task                | Location                      | Notes                                             |
| ------------------- | ----------------------------- | ------------------------------------------------- |
| Modify APS schema   | `schema/aps.schema.ts`        | Always regenerate JSON schema after               |
| Add validation rule | `validation/aps-validator.ts` | Uses Zod for schema validation                    |
| Add gate check      | `gate/checks/`                | Extend BaseCheck, use createSuccess/createFailure |
| Add anti-pattern    | `antipattern/patterns.ts`     | Follow AP-XXX ID convention                       |
| Hash generation     | `crypto/hash.ts`              | Canonical JSON + SHA-256                          |
| Architecture rules  | `architecture/analyzer.ts`    | Uses dependency-cruiser                           |

## Key Abstractions

| Type                   | Purpose                                            |
| ---------------------- | -------------------------------------------------- |
| `APSPlan`              | Hash-stable plan with metadata, changes, lifecycle |
| `GateResult`           | Quality check outcome with timing, score, details  |
| `ValidationResult`     | Structured validation with issues array            |
| `Warning`              | Anti-pattern detection result with location        |
| `ArchitectureBaseline` | Snapshot for violation tracking                    |

## BaseCheck Pattern

All 7 gate checks follow this pattern:

```typescript
import { BaseCheck } from '../check.interface.js';
import type { CheckContext, GateResult } from '../../types/gate.types.js';

export class MyCheck extends BaseCheck {
  readonly name = 'my-check';
  readonly description = 'Validates something important';

  async run(context: CheckContext): Promise<GateResult> {
    const config = context.check_config?.myCheck ?? {};

    // Do validation...
    if (failed) {
      return this.createFailure({
        message: 'What went wrong',
        score: 50,
        details: { issues: [...] },
      });
    }

    return this.createSuccess({
      message: 'All checks passed',
      score: 100,
    });
  }
}
```

Register in `gate/gate-runner.ts` → `registerDefaultChecks()`.

## Gate Checks Reference

| Check        | File                    | Config Key     | Purpose                     |
| ------------ | ----------------------- | -------------- | --------------------------- |
| ESLint       | `eslint.check.ts`       | `eslint`       | Code quality                |
| Coverage     | `coverage.check.ts`     | `coverage`     | Test coverage thresholds    |
| Secret       | `secret.check.ts`       | `secret`       | Pattern + entropy detection |
| Dependency   | `dependency.check.ts`   | `dependency`   | Vulnerability scanning      |
| Architecture | `architecture.check.ts` | `architecture` | Layer violations            |
| Policy       | `policy.check.ts`       | `policy`       | OPA/Rego evaluation         |
| Anti-pattern | `antipattern.check.ts`  | `antipattern`  | Code quality patterns       |

## Hash Generation

Plans are hash-stable via canonical JSON:

```typescript
import { generateHash, canonicalizeJSON } from './crypto/hash.js';

// Sorted keys ensure deterministic output
const canonical = canonicalizeJSON(plan);
const hash = generateHash(canonical); // SHA-256
```

## Scripts

```bash
pnpm -F core run generate:schema        # Zod → JSON Schema
pnpm -F core run update-golden-hashes   # Update test fixtures
nx test core --testNamePattern="gate"   # Test gate checks
```

## Anti-Patterns (This Package)

- Never use `any` in schema definitions
- Never modify hash algorithm without migration plan
- Always use `createSuccess()`/`createFailure()` in checks (never raw objects)
- Always run `generate:schema` after Zod changes

## Testing

- Golden files in `__fixtures__/golden-plans/` for hash verification
- Fixture-based testing for validation scenarios
- Co-located tests with source files
