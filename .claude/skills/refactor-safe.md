# refactor-safe

Perform safe refactoring with continuous validation at each step.

## Parameters

- **scope**: What to refactor (e.g., "rename function", "extract module",
  "simplify logic", "remove duplication")
- **target**: Specific file, function, or symbol to refactor

## Tasks

1. **Establish Baseline** Before making any changes:
   - Run full test suite: `pnpm test`
   - Run typecheck: `pnpm typecheck`
   - Run linter: `pnpm lint:check`
   - Capture results as baseline (all should pass)
   - If baseline fails, fix issues first

2. **Analyse Refactoring Impact**
   - Read the target code to understand:
     - Current implementation
     - Dependencies (what imports it)
     - Dependents (what it imports)
     - Test coverage
   - Search for all usages:
     - Function calls: `Grep` pattern for function name
     - Type references: Search for type name
     - Imports: Find all import statements
   - Estimate blast radius (small/medium/large)

3. **Plan Refactoring Steps** Break refactoring into smallest possible
   increments:
   - **Rename**: Update symbol, update imports, update tests
   - **Extract**: Create new function, replace old code, update tests
   - **Move**: Create new location, update imports, remove old
   - **Simplify**: Reduce complexity one condition at a time
   - Each step should be independently verifiable

4. **Refactor Incrementally** For EACH small change:
   - Make the change (use Edit tool)
   - Run typecheck: `npx nx typecheck <package>`
   - If typecheck fails:
     - Read the error
     - Fix immediately OR revert change
     - Do NOT proceed with more changes
   - Run affected tests: `pnpm test -- <test-file>`
   - If tests fail:
     - Read the failure
     - Fix test or code immediately
     - Do NOT proceed until green

5. **Safety Gates** (run after each increment)

   ```bash
   # Gate 1: TypeScript compilation
   npx nx typecheck <package> || { echo "Revert!"; exit 1; }

   # Gate 2: Tests
   pnpm test -- <affected-files> || { echo "Revert!"; exit 1; }

   # Gate 3: Linting (optional per step, required at end)
   pnpm lint:check || echo "Fix linting before completion"
   ```

6. **Update Tests** After code changes:
   - Update test descriptions if behaviour changed
   - Update mock calls if signatures changed
   - Add new tests if new paths created
   - Remove obsolete tests if code removed
   - Verify test coverage maintained

7. **Verify No Behavioral Changes** (unless intended)
   - Run full test suite: `pnpm test`
   - Check that same tests pass/fail as baseline
   - If behaviour change intended, ensure tests reflect it
   - Look for unintended side effects

8. **Run Full Validation** Before considering refactoring complete:

   ```bash
   # Sync TypeScript references
   npx nx sync

   # Full typecheck
   pnpm typecheck

   # Full test suite
   pnpm test

   # Lint check
   pnpm lint:check

   # Build affected packages
   npx nx build <package>
   ```

9. **Review Changes**
   - Show diff of all changed files
   - Verify changes match intent
   - Check for:
     - Unintended modifications
     - Commented-out code (remove it)
     - Debug statements (remove them)
     - Inconsistent formatting
   - Ensure code quality improved, not degraded

10. **Document If Significant** For major refactorings:
    - Add comment explaining why code structure changed
    - Update module documentation if needed
    - Note any breaking changes
    - Update CHANGELOG if public API affected

## Refactoring Patterns

### Rename Function

```typescript
// Step 1: Create new function with new name
export function newName(...args) {
  return oldName(...args);
}

// ✓ Typecheck, test

// Step 2: Update internal usages to new name
// ✓ Typecheck, test

// Step 3: Mark old function as deprecated
/** @deprecated Use newName instead */
export function oldName(...args) { ... }

// ✓ Typecheck, test

// Step 4: Update all external usages
// ✓ Typecheck, test

// Step 5: Remove old function
// ✓ Typecheck, test
```

### Extract Function

```typescript
// Before
function complex() {
  // ... 50 lines ...
  const x = a + b + c;
  const y = x * 2;
  const z = y + d;
  // ... more code ...
}

// Step 1: Extract to new function
function calculate(a, b, c, d) {
  const x = a + b + c;
  const y = x * 2;
  return y + d;
}
// ✓ Typecheck, test the new function

// Step 2: Use extracted function
function complex() {
  // ... 50 lines ...
  const z = calculate(a, b, c, d);
  // ... more code ...
}
// ✓ Typecheck, test original function still works
```

### Simplify Logic

```typescript
// Before
if (a) {
  if (b) {
    if (c) {
      return x;
    }
  }
}
return y;

// Step 1: Use early returns
if (!a) return y;
if (!b) return y;
if (!c) return y;
return x;
// ✓ Typecheck, test

// Step 2: Combine conditions
if (!a || !b || !c) return y;
return x;
// ✓ Typecheck, test
```

### Remove Duplication

```typescript
// Before: Duplicated code in multiple places

// Step 1: Extract common code to shared function
function shared(params) { ... }
// ✓ Typecheck, test in isolation

// Step 2: Replace first duplication
// ✓ Typecheck, test

// Step 3: Replace second duplication
// ✓ Typecheck, test

// Continue for each duplication
```

## Rollback Strategy

If refactoring gets stuck:

1. **Partial rollback**: Undo last change using git

   ```bash
   git diff  # See what changed
   git checkout -- <file>  # Revert specific file
   ```

2. **Full rollback**: Revert all changes

   ```bash
   git status  # See all changes
   git reset --hard  # Nuclear option
   ```

3. **Restart with smaller steps**: Break refactoring into even smaller
   increments

## Anti-Patterns to Avoid

- ❌ Making multiple changes before running tests
- ❌ "Just one more change" mentality when tests are failing
- ❌ Refactoring while adding features (do separately)
- ❌ Skipping tests because "it's just a rename"
- ❌ Not having a baseline before starting
- ❌ Continuing when not sure what broke

## Example Session

```
Refactoring: Rename parseSpecKit → parseSpecKitV1

1. ✅ Baseline: All tests passing, no type errors

2. 📊 Impact analysis: 12 usages across 5 files

3. 📝 Plan:
   - Create parseSpecKitV1 as alias
   - Update import.ts to use new name
   - Update import-v2.ts to use new name
   - Update tests to use new name
   - Remove old name

4. Step 1: Create alias
   ✓ Typecheck: PASS
   ✓ Tests: PASS (23/23)

5. Step 2: Update import.ts
   ✓ Typecheck: PASS
   ✓ Tests: PASS (23/23)

6. Step 3: Update import-v2.ts
   ✓ Typecheck: PASS
   ✓ Tests: PASS (23/23)

7. Step 4: Update tests
   ✓ Typecheck: PASS
   ✓ Tests: PASS (23/23)

8. Step 5: Remove old function
   ✓ Typecheck: PASS
   ✓ Tests: PASS (23/23)

9. ✅ Final validation:
   ✓ pnpm typecheck: PASS
   ✓ pnpm test: PASS
   ✓ pnpm lint:check: PASS

✅ Refactoring complete: parseSpecKit → parseSpecKitV1
   Changed 5 files, maintained 100% test coverage
```

## Anvil Project Specifics

- Monorepo: Changes may affect multiple packages
- Use `npx nx affected:test` to run only affected tests
- TypeScript strict mode: Catches many refactoring errors
- Project references: Run `npx nx sync` after major changes
- Test framework: Vitest with watch mode available
- Quick validation: `npx nx typecheck <package> && pnpm test -- <file>`
