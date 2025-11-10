---
name: test-audit
description:
  Audit test suite for circular mocking and tests that don't actually test real
  code behavior
---

# Test Audit Command

Identifies tests that mock everything but test nothing - finds circular mocking,
unused handlers, and assertions that only verify mock configuration.

## Usage

```bash
/test-audit
```

Or for a specific file/directory:

```bash
/test-audit path/to/tests
```

## Workflow

1. **Launch test-reality-checker agent** to:
   - Scan test files for circular mocking patterns
   - Identify tests where handlers/functions are never called
   - Find assertions that only verify mock setup
   - Assess risk level of false-passing tests

2. **Report findings** with:
   - Summary of suspicious tests
   - Detailed breakdown per test
   - Suggested fixes
   - Risk assessment

3. **Optional: Fix tests** - if user approves, rewrite tests to:
   - Actually call the code under test
   - Mock only external dependencies
   - Assert on real behavior

## What This Finds

- Tests that set `reply.send(X)` then assert `reply.send` was called with `X`
- Tests that configure mocks but never call the actual handler
- Tests where mock return values appear directly in assertions
- Tests that would still pass if you deleted the implementation

## Output

A report showing which tests are giving false confidence and how to fix them.
