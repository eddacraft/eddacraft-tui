# fix-build

Systematically fix TypeScript build and type errors in the monorepo.

## Parameters
- **package_name**: (Optional) Specific package to fix (e.g., "adapters", "cli"), or "all" for entire workspace

## Tasks

1. **Sync TypeScript Project References**
   - Run `npx nx sync` to update project references
   - This ensures tsconfig.json files have correct references
   - Check for sync issues: `npx nx sync:check`

2. **Run Type Check and Capture Errors**
   - If `package_name` specified:
     - `npx nx typecheck ${package_name} > typecheck.log 2>&1 || true`
   - If "all" or no package:
     - `pnpm typecheck > typecheck.log 2>&1 || true`
   - Parse and categorize errors

3. **Group Errors by Type**
   Organize errors into categories:
   - **Import errors**: Cannot find module, no exported member
   - **Type mismatches**: Type X is not assignable to type Y
   - **Missing types**: Property X does not exist on type Y
   - **Null/undefined**: Object is possibly undefined
   - **Implicit any**: Parameter has implicitly any type
   - **Generic constraints**: Type doesn't satisfy constraint

4. **Fix Priority Order** (highest impact first)
   - **Blocking errors**: Import failures that cascade
   - **Interface changes**: Type definition updates needed
   - **Type assertions**: Add proper types to function signatures
   - **Strict mode issues**: Null checks, any types
   - **Unused code**: Remove if safe, otherwise add ignore

5. **Fix Import Errors First**
   - Check if imported module exists
   - Verify import path is correct (relative vs @anvil/*)
   - Ensure package is built: `npx nx build <package>`
   - Check package.json exports field
   - Update import statement if API changed

6. **Fix Type Mismatches**
   - Read both type definitions to understand mismatch
   - Check if upstream type changed (breaking change)
   - Options:
     - Update code to match new type
     - Add type adapter/transformer
     - Use type assertion if safe: `as NewType`
     - Fix upstream type if incorrect

7. **Fix Missing Properties**
   - Check if property was renamed or removed
   - Add optional chaining: `obj?.property`
   - Provide defaults: `obj.property ?? defaultValue`
   - Update interface if property should exist

8. **Fix Null/Undefined Issues**
   - Add null checks before usage:
     ```typescript
     if (!value) throw new Error('Value required');
     // Now TypeScript knows value is defined
     ```
   - Use optional chaining: `obj?.method?.()`
   - Use nullish coalescing: `value ?? default`
   - Update function signatures to allow null if appropriate

9. **Re-run Type Check After Each Category**
   - Fix all errors in one category
   - Run typecheck again
   - Verify errors are resolved
   - Move to next category
   - This prevents fixing errors that don't exist after earlier fixes

10. **Verify Build Success**
    - Run full typecheck with no errors
    - Build affected packages: `npx nx build ${package_name}`
    - Run tests to ensure no runtime breakage: `pnpm test`
    - Report summary of fixes made

## Common TypeScript Issues in Anvil

### Workspace Protocol Issues
```typescript
// Problem: Can't resolve @anvil/* imports
// Fix: Ensure package is built first
npx nx build core  // Build dependency first
npx nx build adapters  // Then dependent package
```

### Type Exports Not Found
```typescript
// Problem: Module '"@anvil/core"' has no exported member 'APS'
// Fix: Check package.json exports and tsconfig
// core/package.json:
"exports": {
  "./schema": "./src/schema/index.ts"
}

// Import correctly:
import type { APS } from '@anvil/core/schema';
```

### Strict Mode Violations
```typescript
// Problem: Object is possibly 'undefined'
const title = artifact.metadata.title.toUpperCase();

// Fix: Add null check
const title = artifact.metadata?.title?.toUpperCase() ?? 'UNTITLED';
```

### Discriminated Union Issues
```typescript
// Problem: Property 'X' does not exist on type 'A | B'
function handle(item: TypeA | TypeB) {
  console.log(item.specificField);  // Error!
}

// Fix: Use type narrowing
function handle(item: TypeA | TypeB) {
  if ('specificField' in item) {
    console.log(item.specificField);  // OK
  }
}
```

### Generic Constraints
```typescript
// Problem: Type 'T' does not satisfy constraint 'HasId'
function process<T>(items: T[]): void {
  items.forEach(item => console.log(item.id));  // Error
}

// Fix: Add constraint
function process<T extends HasId>(items: T[]): void {
  items.forEach(item => console.log(item.id));  // OK
}
```

## Build Order in Anvil

Packages have dependencies that require specific build order:

```
1. core         (no dependencies)
2. adapters     (depends on core)
3. gate         (depends on core)
4. cli          (depends on core, adapters, gate)
5. ui           (depends on core)
```

If build fails, rebuild in dependency order:
```bash
npx nx build core && \
npx nx build adapters && \
npx nx build gate && \
npx nx build cli
```

## Output Format

Show clear summary of fixes:

```
🔧 TypeScript Build Fixes

📦 Package: @anvil/adapters

Errors fixed:
  ✓ 8 import errors (missing @anvil/core exports)
  ✓ 3 type mismatches (APS schema updated)
  ✓ 5 null safety issues (added optional chaining)

Total: 16 errors resolved

✅ Build successful: npx nx build adapters
✅ Tests passing: pnpm test -- packages/adapters
```

## Anvil Project Specifics

- TypeScript strict mode enabled in `tsconfig.base.json`
- Project references used for incremental builds
- Use `npx nx sync` before major refactoring
- Run `npx nx graph` to visualize dependencies
- All packages under `@anvil/*` namespace
- ES2022 target with ESM modules
