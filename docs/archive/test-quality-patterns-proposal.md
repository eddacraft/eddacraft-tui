# Test Quality Anti-Patterns (Proposed AP-008 through AP-015)

Candidate patterns for future inclusion in Anvil's anti-pattern catalogue. These
were identified during a comprehensive audit of the anvil test suite where they
masked real bugs and allowed broken code to pass CI undetected.

> **Category**: A new `test-quality` category should be added to
> `AntiPatternSchema` alongside the existing `escape-hatch`, `error-handling`,
> `code-quality`, and `type-safety` categories.
>
> **Scope**: These patterns should only scan test files (`*.test.ts`,
> `*.spec.ts`, `**/__tests__/**`).

---

## AP-008: Fake assertion (`expect(true).toBe(true)`)

**Severity**: error | **Confidence**: high | **Detection**: regex

A test that asserts a hardcoded literal against itself provides zero coverage.
It exists only to inflate test counts and will never fail regardless of code
behaviour.

```
// Bad
it('should validate input', () => {
  try {
    validate(input);
    expect(true).toBe(true);   // <-- always passes
  } catch {
    expect(true).toBe(true);   // <-- also always passes
  }
});
```

**Regex**:

```
expect\s*\(\s*true\s*\)\s*\.\s*toBe\s*\(\s*true\s*\)
```

Also match the `false`/`false` variant:

```
expect\s*\(\s*false\s*\)\s*\.\s*toBe\s*\(\s*false\s*\)
```

**Real example found**: `opa-binary-manager.test.ts` had 11 tests where both the
try and catch branches contained `expect(true).toBe(true)`, meaning the test
passed whether the code threw or not.

---

## AP-009: Try/catch swallowing assertion failures in tests

**Severity**: error | **Confidence**: medium | **Detection**: regex (test files
only)

A `try/catch` block in a test that catches all errors will also catch
Vitest/Jest assertion errors (`AssertionError`), silently swallowing test
failures. The test passes even when assertions fail.

```
// Bad
it('should commit with provenance', () => {
  try {
    const result = getProvenance();
    expect(result.commit).toBeDefined();
    expect(result.author).toBe('test');   // if this fails, catch eats it
  } catch {
    // test passes silently
  }
});
```

**Regex** (in test files):

```
catch\s*\([^)]*\)\s*\{[\s\S]*?\}
```

Note: This needs to be scoped to test files and may require AST analysis for
accuracy, since legitimate test `catch` blocks exist (e.g., testing that code
throws). A more precise heuristic: flag `catch` blocks in test files that
contain `expect()` calls _before_ the catch (i.e., the `expect` is in the `try`,
not testing a thrown error).

**Real example found**: `provenance.test.ts` had two git-dependent tests wrapped
in try/catch. When git signing failed, the catch silently ate the error. This
also hid a real bug in `collector.ts` where `substring(3)` on trimmed porcelain
output produced truncated filenames.

---

## AP-010: No-op numeric assertion (`toBeGreaterThanOrEqual(0)` on length)

**Severity**: error | **Confidence**: high | **Detection**: regex

Asserting that an array's `.length` is `>= 0` is a tautology — array length is
always non-negative. The assertion can never fail.

```
// Bad — always passes
expect(results.length).toBeGreaterThanOrEqual(0);

// Good — asserts actual expectation
expect(results).toHaveLength(3);
```

**Regex**:

```
expect\s*\([^)]*\.length\s*\)\s*\.\s*toBeGreaterThanOrEqual\s*\(\s*0\s*\)
```

**Real example found**: `layer-detector.test.ts` tested ambiguous layer
detection with `expect(ambiguous.length).toBeGreaterThanOrEqual(0)`. The test
passed even when the detector returned zero results.

---

## AP-011: forEach over potentially empty array hiding assertions

**Severity**: warning | **Confidence**: medium | **Detection**: regex +
heuristic

When a test iterates over an array with `.forEach()` and all assertions live
inside the callback, an empty array means zero iterations and zero assertions
executed. The test passes vacuously.

```
// Bad — if violations is [], no assertions run
violations.forEach((v) => {
  expect(v.severity).toBe('error');
  expect(v.message).toBeDefined();
});

// Good — assert non-empty first, then iterate
expect(violations.length).toBeGreaterThan(0);
violations.forEach((v) => {
  expect(v.severity).toBe('error');
});

// Also good — use test.each or assert specific indices
expect(violations[0].severity).toBe('error');
```

**Regex** (heuristic — flag for review):

```
\.\s*forEach\s*\(\s*\([^)]*\)\s*=>\s*\{[^}]*expect\s*\(
```

This flags `forEach` callbacks that contain `expect()`. It should be combined
with a check that there is no preceding length assertion on the same variable.

**Real example found**: `opa-executor.test.ts` (runtime) used `forEach` over a
`violations` array to assert fields. When the mock OPA binary returned no
violations, the loop never ran and the test passed with zero assertions.

---

## AP-012: Trivial constructor-only test

**Severity**: warning | **Confidence**: medium | **Detection**: regex

A test that only instantiates an object and checks `toBeDefined()` or
`toBeInstanceOf()` without exercising any behaviour is effectively testing that
the JavaScript `new` keyword works.

```
// Bad — tests nothing useful
it('should create instance', () => {
  const detector = new EntryDetector(config);
  expect(detector).toBeDefined();          // trivial
  expect(detector).toBeInstanceOf(EntryDetector); // also trivial
});

// Good — tests actual behaviour
it('should detect package entry points', () => {
  const detector = new EntryDetector(config);
  const result = detector.detectEntryPoint('src/index.ts');
  expect(result.type).toBe('package');
});
```

**Regex** (heuristic — flag `it` blocks where the only `expect` uses
`toBeDefined` or `toBeInstanceOf`):

```
it\s*\([^)]*,\s*(?:async\s*)?\(\)\s*=>\s*\{[^}]*expect\s*\([^)]*\)\s*\.\s*(?:toBeDefined|toBeInstanceOf)\s*\([^)]*\)\s*;?\s*\}
```

This is a simplified regex. AST-based detection would be more reliable: flag
`it`/`test` blocks where every `expect` call uses only `toBeDefined`,
`toBeInstanceOf`, or `not.toBeNull`.

**Real example found**: `entry-detector.test.ts`, `snapshot-capture.test.ts`,
`bundle-manager.test.ts`, and `bundle-verifier.test.ts` all had tests that only
called the constructor and asserted `toBeDefined()`.

---

## AP-013: Weak value assertion (`toBeTruthy`/`toBeDefined` on data)

**Severity**: info | **Confidence**: medium | **Detection**: regex

Using `toBeTruthy()` or `toBeDefined()` on a value that has a known expected
shape provides much weaker coverage than asserting the actual value. These
assertions pass for any non-nullish value, including wrong values.

```
// Bad — passes even if name is 'wrong_value'
expect(pattern.name).toBeTruthy();

// Good — checks the actual value
expect(pattern.name).toBe('empty-catch-block');

// Bad — passes for any object including {}
expect(result.location).toBeDefined();

// Good — checks the shape
expect(result.location).toEqual({ file: 'src/app.ts', line: 42 });
```

**Regex**:

```
expect\s*\([^)]+\)\s*\.\s*(?:toBeTruthy|toBeDefined)\s*\(\s*\)
```

Note: This is the noisiest pattern. Consider making it opt-in (`optIn: true`) or
limiting scope to test files where specific value assertions are clearly
possible.

**Real example found**: `patterns.test.ts` used `toBeTruthy()` for all pattern
field assertions. `scanner.test.ts` used `toBeDefined()` on warning severity,
confidence, title, and location fields instead of checking actual values.

---

## AP-014: Conditional assertion in test (`if` guarding `expect`)

**Severity**: warning | **Confidence**: high | **Detection**: regex

An `if` statement guarding an `expect()` call means the assertion may be
silently skipped. If the condition is false, the test passes without running the
assertion.

```
// Bad — if result.success is false, the inner assertions never run
if (result.success) {
  expect(result.data).toBeDefined();
  expect(result.data.raw_output).toContain('allow');
}

// Good — assert the condition, then check the value
expect(result.success).toBe(true);
expect(result.data.raw_output).toContain('allow');
```

**Regex** (in test files):

```
if\s*\([^)]+\)\s*\{[^}]*expect\s*\(
```

This will have some false positives (e.g., helper functions that intentionally
branch). AST analysis scoped to `it`/`test` block bodies would improve
precision.

**Real example found**: `opa-executor.test.ts` had `if (result.success)` guards
around assertions on `raw_output`. When the mock binary wasn't found and
`result.success` was `false`, all assertions inside were skipped.

---

## AP-015: Circular mock (mocking the module under test)

**Severity**: error | **Confidence**: low | **Detection**: AST preferred

When a test mocks private methods or internal behaviour of the unit under test,
it ends up testing the mock rather than the real code. Changes to the real
implementation go undetected.

```
// Bad — mocking the thing you're testing
vi.spyOn(binaryManager as any, 'downloadBinary').mockResolvedValue('/fake/path');
vi.spyOn(binaryManager as any, 'getDownloadUrl').mockReturnValue('http://fake');
const result = await binaryManager.ensureBinary();
// This only tests that ensureBinary calls downloadBinary — not that downloading works

// Good — mock external dependencies, not internals
vi.spyOn(fs, 'existsSync').mockReturnValue(false);
vi.spyOn(childProcess, 'execSync').mockReturnValue(Buffer.from('downloaded'));
const result = await binaryManager.ensureBinary();
```

**Detection**: This is difficult to detect with regex alone. Recommended
approach:

1. **Regex heuristic** (flag for review): `vi\.spyOn\s*\(\s*\w+\s+as\s+any` —
   casting to `any` before spying suggests mocking private internals
2. **AST-based** (preferred): detect when `vi.spyOn` target is the same variable
   that is the subject of `expect()` assertions in the same test

**Real example found**: `opa-binary-manager.test.ts` mocked `downloadBinary`,
`getDownloadUrl`, and `verifyBinary` — all private methods of the class under
test — then asserted that calling the public method invoked them. The tests were
testing the mock wiring, not the actual download/verify logic.

---

## Implementation Notes

### New category required

The `AntiPatternSchema` category enum in `types.ts` needs a new value:

```typescript
category: z.enum([
  'escape-hatch',
  'error-handling',
  'code-quality',
  'type-safety',
  'test-quality', // <-- new
]);
```

### File scope

All test-quality patterns should have an **inverse** allowlist — they should
_only_ run on test files. This could be implemented as a new `targetFiles`
field:

```typescript
targetFiles: ['**/*.test.ts', '**/*.spec.ts', '**/__tests__/**'];
```

Or by inverting the existing `allowlist` logic for these patterns.

### Detection method priority

| Pattern | Regex feasibility          | AST recommended |
| ------- | -------------------------- | --------------- |
| AP-008  | High (exact match)         | No              |
| AP-009  | Medium (needs context)     | Yes             |
| AP-010  | High (exact match)         | No              |
| AP-011  | Low (needs scope analysis) | Yes             |
| AP-012  | Low (needs block analysis) | Yes             |
| AP-013  | High (exact match)         | No              |
| AP-014  | Medium (false positives)   | Yes             |
| AP-015  | Low (needs data flow)      | Yes             |

### Severity rationale

- **error** (AP-008, AP-009, AP-010, AP-015): These patterns produce tests that
  _can never fail_ or _test the wrong thing_. They actively hide bugs.
- **warning** (AP-011, AP-012, AP-014): These patterns produce tests that _might
  not fail_ depending on runtime data. They represent risk.
- **info** (AP-013): Weak assertions still provide some coverage. This is more
  of a code quality signal than a correctness issue.
