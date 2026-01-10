<!-- APS: See https://github.com/EddaCraft/anvil-plan-spec for format reference -->

# Test Quality Enforcement

## Overview

Automated enforcement of testing best practices as documented in
`docs/TESTING.md`. This module provides:

1. An audit script for detecting violations
2. Custom ESLint rules for ongoing enforcement
3. Guidance for fixing existing violations

## Problem

Tests that don't follow best practices lead to:

- **Test pollution** - Tests that pass in isolation but fail when run together
- **Brittle tests** - Over-mocking hides real bugs
- **Maintenance burden** - `as any` casts bypass type safety
- **Flaky CI** - Global state leaks between tests

## Success Criteria

- [x] Audit script detects all violation types
- [x] ESLint rules catch violations at lint time
- [ ] All existing violations fixed or acknowledged
- [ ] New violations blocked in CI (rules promoted to `error`)

## Deliverables

### Audit Script (`scripts/audit-tests.ts`)

Scans all test files and detects:

- `as any` type assertions
- Missing `afterEach` cleanup with `vi.restoreAllMocks()`
- `process.chdir()` without restoration
- Temp directory creation without cleanup

### ESLint Plugin (`packages/eslint-plugin-anvil/`)

Custom rules (set to `warn` initially):

| Rule                         | Description                                     |
| ---------------------------- | ----------------------------------------------- |
| `anvil/no-any-in-tests`      | Disallow `as any` in test files                 |
| `anvil/require-mock-cleanup` | Require `vi.restoreAllMocks()` when mocking     |
| `anvil/require-cwd-restoration` | Require restoring `process.cwd()` after chdir |

## Tasks

| Task     | Description                      | Status   | Priority |
| -------- | -------------------------------- | -------- | -------- |
| TEST-001 | Create audit script              | Complete | high     |
| TEST-002 | Create ESLint plugin package     | Complete | high     |
| TEST-003 | Integrate plugin into lint       | Complete | high     |
| TEST-004 | Fix `as any` violations          | Planned  | medium   |
| TEST-005 | Fix mock cleanup violations      | Planned  | medium   |
| TEST-006 | Fix cwd restoration violations   | Planned  | medium   |
| TEST-007 | Promote rules to `error`         | Planned  | low      |

## Current Violations (Baseline)

Audit run on 2025-01-10:

- **Total violations:** 28
- **Files affected:** 11
- **By rule:**
  - `no-any-in-tests`: 23
  - `require-mock-cleanup`: 3
  - `require-cwd-restoration`: 2

### Files Requiring Fixes

| File                                              | Violations |
| ------------------------------------------------- | ---------- |
| `core/src/gate/checks/secret.check.test.ts`       | 8          |
| `core/src/gate/checks/policy.check.test.ts`       | 6          |
| `packages/adapters/src/__tests__/speckit-export.test.ts` | 5     |
| `core/src/validation/aps-validator.test.ts`       | 2          |
| `cli/src/__tests__/cli-aps-integration.test.ts`   | 1          |
| `cli/src/__tests__/cli-gate-integration.test.ts`  | 1          |
| `cli/src/__tests__/cli-speckit-integration.test.ts` | 1        |
| `cli/src/commands/hooks.test.ts`                  | 1          |
| `cli/src/tui/utils/__tests__/tty-detection.test.ts` | 1        |
| `core/src/gate/checks/coverage.check.test.ts`     | 1          |
| `packages/adapters/src/__tests__/speckit-import-v2.test.ts` | 1 |

## Dependencies

None — this module enforces existing guidelines from `docs/TESTING.md`.

## Related

- `docs/TESTING.md` - Testing best practices documentation
- `eslint.config.mjs` - ESLint configuration
