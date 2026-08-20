# Testing Best Practices

| Type  | Authority     | Owner | Status | Freshness                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ----- | ------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Guide | Authoritative | TEST  | Live   | Last reviewed 2026-08-20 against `Cargo.toml` (workspace version `0.9.6-beta`; animation crate exact-pinned `animate-core =0.4.1`) and earlier `.github/workflows/rust-tests.yml` (CIB-338 made the paths-filter gate on the required `Test` check fail open — documented under CI Flake Triage), `.github/workflows/ci.yml` (CIB-335 parity step), `.github/workflows/rust.yml`, `AGENTS.md`, `plans/project-context.md`, and AICON-002 |

| Upstream                                                                                                                                                                                                      | Downstream                                                                              |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `AGENTS.md`, `plans/project-context.md`, `package.json`, `Cargo.toml`, `apps/e2e/vitest.config.ts`, `.github/workflows/ci.yml`, `.github/workflows/rust.yml`, `.github/workflows/rust-tests.yml`, `policies/` | `docs/guides/README.md`, `AGENTS.md`, `pnpm test`, `cargo test --workspace`, `opa test` |

This guide covers testing conventions and best practices for the Anvil monorepo.
TypeScript packages use **Vitest**; Rust crates use **cargo test** with
**insta** (snapshot testing) and **criterion** (benchmarks).

`AGENTS.md` intentionally links here instead of carrying the full test
catalogue. This guide owns stack-specific command selection, E2E conventions,
coverage notes, and OPA/Regal policy-test notes.

## Test Stack

| Stack           | Location                                              | Runner                                             |
| --------------- | ----------------------------------------------------- | -------------------------------------------------- |
| TypeScript unit | `packages/**/__tests__`, co-located `*.test.ts` files | Vitest via Nx                                      |
| Rust unit       | `crates/**/src/**/tests`, co-located module tests     | `cargo test`                                       |
| E2E             | `apps/e2e/src/**/*.e2e.test.ts`                       | Vitest workspace using `apps/e2e/vitest.config.ts` |
| Rego policy     | `policies/fixtures/*.rego` and policy package tests   | `opa test`; Regal for linting where configured     |

## Quick Reference

```bash
# TypeScript
pnpm test                    # Run all unit tests
pnpm test:coverage           # With coverage reports
pnpm test:e2e:harness        # Vitest E2E harness
npx nx test core             # Test specific package
npx nx test adapters --testNamePattern="BMAD"  # Run matching tests

# Rust
cargo test --workspace       # Run all Rust tests
cargo test -p eddacraft-anvil-kernel   # Test specific crate
cargo test -p eddacraft-anvil-checks -- secret  # Filter by test name
cargo insta review           # Review snapshot changes
cargo bench -p eddacraft-anvil-checks  # Run criterion benchmarks

# Rego / policy checks
opa test policies/fixtures   # Run policy fixture tests when OPA is installed
```

---

## Test Organisation

### File Naming & Location

- **Co-locate tests with source code** using `.test.ts` extension
- Use `__fixtures__/` directories for test data
- Use `__tests__/helpers/` for shared test utilities

```
src/
├── validation/
│   ├── aps-validator.ts
│   └── aps-validator.test.ts    # Co-located test
├── __fixtures__/
│   └── golden-plans/            # Fixture data
└── __tests__/
    └── helpers/                 # Shared utilities
        └── test-workspace.ts
```

### Standard Imports

```typescript
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
```

---

## Mocking Guidelines

### 1. Default stance: don't mock unless you must

Start with real code. Mock only when the "real thing" is **slow, flaky,
non-deterministic, expensive, or outside your control**.

**Good reasons to mock:**

- Network (HTTP clients, fetch/axios)
- Databases / queues / caches
- Filesystem (for unit tests)
- Time (`Date.now`, timers), randomness
- OS/process (`process.env`, `process.cwd`, `process.exit`)
- Third-party SDKs

### 2. Mock at _your boundary_, not deep inside vendors

Create small wrapper modules you own (e.g. `src/http.ts`, `src/db.ts`,
`src/clock.ts`). Mock those wrappers in tests rather than mocking `fetch`,
vendor SDKs, or internals.

**Why:** The seam stays stable even if the vendor API changes.

### 3. Don't mock the behaviour you're trying to learn about

Avoid mocking your core domain logic (calculations, business rules,
decision-making). It's fine to mock _collaborators_ so you can test domain logic
in isolation.

### 4. Choose the right kind of mock

| Tool                      | Use Case                                                     |
| ------------------------- | ------------------------------------------------------------ |
| `vi.fn()`                 | Plain function stubs                                         |
| `vi.spyOn(obj, 'method')` | Real implementation + assert calls (or temporarily override) |
| `vi.mock('module')`       | Replace entire module imports                                |

**Rule of thumb:** Prefer **spy** over **full module mock** when you only need
to intercept one method.

### 5. Keep mocks local, explicit, and boring

Set up the mock in the test file (or a small helper) where it's used. Avoid
"global mocks everywhere" unless you're doing a deliberate test environment
shim.

### 6. Reset correctly to avoid test pollution

Use one of these patterns consistently:

```typescript
// For spies (vi.spyOn)
afterEach(() => {
  vi.restoreAllMocks();
});

// For vi.fn() and module mocks
beforeEach(() => {
  vi.resetAllMocks();
});
```

**Pick one pattern and stick to it across the repo.**

### 7. Prefer typed mocks over `as any`

```typescript
// Good - typed mock access
import { myModule } from './my-module.js';
vi.mock('./my-module.js');
const mocked = vi.mocked(myModule);
mocked.someMethod.mockReturnValue('test');

// Avoid - loses type safety
(myModule as any).someMethod.mockReturnValue('test');
```

### 8. Test outcomes first; assert interactions only when meaningful

**Prioritise:**

- Returned values
- Thrown errors
- State changes

**Assert interactions when the behaviour includes:**

- Correct arguments to dependency calls
- Correct number/order of calls
- Idempotency/retry behaviour

### 9. Keep test data "minimum realistic"

Use objects shaped like real data, but only include fields that matter. Prefer
small factories/builders over giant fixtures.

```typescript
// Good - minimal but realistic
function createMinimalPlan(overrides = {}): APSPlan {
  return {
    id: 'PLN-001',
    schema_version: '0.1.0',
    title: 'Test Plan',
    ...overrides,
  };
}

// Avoid - massive fixture with every field
const plan = require('./fixtures/full-plan-with-everything.json');
```

### 10. Time and async: be deliberate

- If time matters, use fake timers
- When mixing timers + promises, use async timer helpers to avoid hanging tests
- Don't fake timers globally unless you must; scope to the tests that need it

```typescript
it('should expire cache entries', async () => {
  vi.useFakeTimers();

  await cache.set('key', 'value', { ttl: 1000 });

  vi.advanceTimersByTime(1001);

  expect(await cache.get('key')).toBeNull();

  vi.useRealTimers();
});
```

### 11. Avoid heavy global state mocking

Prefer injecting config/clock dependencies or wrapping them in modules. If you
must mutate globals (env, Date), restore them in `afterEach`.

```typescript
let originalEnv: string | undefined;

beforeEach(() => {
  originalEnv = process.env.NODE_ENV;
  process.env.NODE_ENV = 'test';
});

afterEach(() => {
  process.env.NODE_ENV = originalEnv;
});
```

### 12. Backstop with integration tests

Have a small number of tests that:

- Run the real module wiring end-to-end with minimal mocks
- Use temp directories for filesystem operations
- Catch "everything works in mocks-land" bugs

---

## Anvil-Specific Patterns

### E2E Harness

E2E tests live under `apps/e2e/src/**/*.e2e.test.ts` and run through the Vitest
workspace config at `apps/e2e/vitest.config.ts`. Keep E2E tests focused on
observable CLI, daemon, hook, or workflow behaviour that cannot be proven with a
unit test alone. Use isolated HOME/runtime directories and temporary workspaces
so tests do not mutate the developer's real Anvil, Git, or editor state.

### Coverage

Use `pnpm test:coverage` when a change needs coverage evidence. Coverage output
is evidence, not a design target: prefer meaningful boundary and regression
tests over assertions written only to raise a percentage.

### OPA And Regal

Policy fixtures live under `policies/fixtures/`. Use `opa test` for Rego policy
behaviour and Regal where the policy surface has lint configuration. Keep policy
fixtures small and name them after the behaviour they prove so failed checks map
back to the policy intent quickly.

### Golden File Testing (Hash Stability)

For deterministic hashing verification, use golden files:

```typescript
// Example: golden-files.test.ts (co-located with golden file fixtures)
import { goldenPlans } from './__fixtures__/golden-plans/index.js';

describe('Golden Files', () => {
  it.each(Object.entries(goldenPlans))(
    'should maintain stable hash for %s',
    async ([name, plan]) => {
      const { hash: expectedHash, ...planWithoutHash } = plan;
      const actualHash = generateHash(planWithoutHash);
      expect(actualHash).toBe(expectedHash);
    }
  );
});
```

### Fixture Loading (ESM Pattern)

```typescript
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join } from 'node:path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const fixturesDir = join(__dirname, 'fixtures');

const content = await readFile(join(fixturesDir, 'sample.md'), 'utf-8');
```

### Test Workspace Helper

Use the shared helper for CLI and integration tests:

```typescript
import {
  createTestWorkspace,
  createMinimalAPSPlan,
  type TestWorkspace,
} from '../helpers/test-workspace.js';

describe('CLI Command', () => {
  let workspace: TestWorkspace;
  const originalCwd = process.cwd();

  beforeEach(() => {
    workspace = createTestWorkspace();
    process.chdir(workspace.root);
  });

  afterEach(() => {
    process.chdir(originalCwd);
    workspace.cleanup();
    vi.restoreAllMocks();
  });

  it('should validate plan', async () => {
    const plan = createMinimalAPSPlan({ title: 'Test' });
    // ... test logic
  });
});
```

### Temporary Directory Pattern

```typescript
import { mkdirSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

describe('Filesystem Operations', () => {
  let tempDir: string;
  const originalCwd = process.cwd();

  beforeEach(() => {
    tempDir = join(tmpdir(), 'anvil-test', Math.random().toString(36));
    mkdirSync(tempDir, { recursive: true });
    process.chdir(tempDir);
  });

  afterEach(() => {
    process.chdir(originalCwd);
    if (existsSync(tempDir)) {
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});
```

---

## Package-Specific Guidance

### Core (`packages/anvil/core/`)

- **Focus:** Schema validation, hashing, gate checks
- **Pattern:** Heavy use of fixtures, determinism verification
- **Key tests:** `golden-files.test.ts`, gate check tests

```typescript
// Testing validation
it('should reject invalid plan ID format', () => {
  const result = APSPlanSchema.safeParse({
    id: 'invalid-id', // Wrong format
    // ...
  });
  expect(result.success).toBe(false);
});
```

#### Compiled pattern registry parity (CIB-335)

`patterns/compiled/registry.json` is generated from the `.anvil` sources and
embedded into the Rust scanner binary with `include_str!`, so a stale registry
ships a rule set that no longer matches `patterns/`. The `Unit Tests` job runs
`pnpm --filter @eddacraft/anvil-core patterns:check` to catch that.

Two things to know when touching the compiler or a rule:

- **Regenerate, never hand-edit** `registry.json`. Run
  `pnpm --filter @eddacraft/anvil-core patterns:compile`, then let the
  pre-commit hook normalise it — the compiler writes expanded JSON while the
  committed file is formatter-normalised.
- **Warnings are not drift.** The nine legacy `AP`-prefix collision warnings are
  emitted by design on every run and do not fail the check. Pass `--strict` to
  escalate warnings to failures locally.

The check compares against a fresh compile with object keys canonicalised, so
key reordering is not drift; array order _is_ significant, because the compiler
sorts patterns and families for stable diffs.

`patterns/**` has no dedicated class in the CI change classifier — it reaches
`unit-tests-required` through the conservative `unknown` fallback. That coupling
is incidental, so it is pinned by fixtures in
`scripts/ci/classify-changes.test.sh`; if you add a `patterns` path class,
update those fixtures or this gate silently stops running.

### Adapters (`packages/adapters/`)

- **Focus:** Format detection, parse/serialise round-trips
- **Pattern:** Fixture-heavy, test confidence scoring

```typescript
// Testing format detection
it('should detect valid spec with high confidence', async () => {
  const content = await readFile(join(fixturesDir, 'valid-spec.md'), 'utf-8');
  const result = adapter.detect(content);

  expect(result.detected).toBe(true);
  expect(result.confidence).toBeGreaterThanOrEqual(50);
  expect(result.reasons).toContain('Has required frontmatter');
});
```

### CLI (`anvil-archive/anvil-cli-node/` — legacy TypeScript)

- **Focus:** Command structure, argument parsing, user interaction
- **Pattern:** Mock external deps (inquirer, ora, chalk), use test workspaces

> **Note:** The primary CLI is now the Rust binary at `crates/anvil-cli/`. This
> section covers the archived Node.js CLI retained for historical reference.

```typescript
// Testing CLI commands
vi.mock('inquirer', () => ({
  default: { prompt: vi.fn() },
}));

vi.spyOn(process, 'exit').mockImplementation(() => undefined as never);
vi.spyOn(console, 'log').mockImplementation(() => {});
```

---

## Rust Testing

### Running Tests

```bash
cargo test --workspace                      # All crates
cargo test -p eddacraft-anvil-kernel                  # Single crate
cargo test -p eddacraft-anvil-checks -- secret        # Filter by name
INSTA_UPDATE=1 cargo test -p eddacraft-anvil-kernel   # Update snapshots
cargo insta review                          # Interactive snapshot review
```

### Snapshot Testing (insta)

Rust crates use [insta](https://insta.rs/) for snapshot testing. Snapshots are
stored alongside test files and committed to version control.

```rust
use insta::assert_yaml_snapshot;

#[test]
fn parses_symbol_graph() {
    let graph = parse_file("fixtures/sample.ts");
    assert_yaml_snapshot!(graph);
}
```

When a snapshot changes, `cargo insta review` launches an interactive TUI to
accept or reject the diff.

### Benchmarks (criterion)

Performance-critical crates (`anvil-checks`, `anvil-kernel`, `anvil-bench`) use
[criterion](https://bheisler.github.io/criterion.rs/) for benchmarks.

```bash
cargo bench -p eddacraft-anvil-checks                 # Run check benchmarks
cargo bench -p eddacraft-anvil-kernel                 # Run kernel benchmarks
```

Benchmark results are output as HTML reports in `target/criterion/`.

### Workspace Policies

- `unsafe_code = "forbid"` — no unsafe code allowed
- `clippy all = "deny"` — all clippy warnings are errors
- All kernel errors are structured events — no panics across boundaries

---

## CI Flake Triage

Not every red X is the diff's fault. Known infrastructure failure classes
(CIB-338):

- **`Set up job` failures are runner provisioning.** They are external and never
  attributable to the diff. Rerun only the failed jobs with
  `gh run rerun <run-id> --failed`.
- **Path-detection outages fail open.** The required `Test` check
  (`.github/workflows/rust-tests.yml`) gates its heavy steps behind a
  `dorny/paths-filter` step; since CIB-338 a filter failure (`continue-on-error`
  plus an outcome check in the gate step) runs the full Rust gate instead of
  redding the job with every step skipped. A red `Test` therefore means real
  work failed, not that detection died. The contract is pinned by
  `scripts/ci/rust-tests-fail-open.test.sh`.
- **Vitest pool crash — watch-only.** Signature:
  `[vitest-pool]: Worker forks emitted error … Worker exited unexpectedly` in
  the `anvil-source:test` task with all test files passing (first seen in run
  31782134751). Not yet reproduced; no worker or memory stabiliser is applied —
  the root `vitest.config.ts` sets no `poolOptions` and no repo convention caps
  `maxForks`, so any tuning would be a guess. If it recurs, capture the run id
  and runner memory context before changing pool settings.

---

## Quick Checklist

Before submitting tests, verify:

- [ ] Mock only slow/flaky/external boundaries
- [ ] Mock at your boundary wrapper modules
- [ ] Restore spies in `afterEach`
- [ ] Reset/clear mocks consistently
- [ ] Use `vi.mocked()` for TypeScript typing
- [ ] Assert outcomes; assert interactions only when behaviour depends on them
- [ ] Use fake timers only where needed (async helpers when promises involved)
- [ ] Clean up temp directories and restore `process.cwd()`
- [ ] Include integration tests to catch wiring issues
