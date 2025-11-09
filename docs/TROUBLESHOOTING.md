# Anvil Troubleshooting Guide

Common issues and solutions for using Anvil.

## Table of Contents

- [Installation Issues](#installation-issues)
- [Format Detection Issues](#format-detection-issues)
- [Validation Errors](#validation-errors)
- [Gate Failures](#gate-failures)
- [Export/Conversion Issues](#exportconversion-issues)
- [Performance Issues](#performance-issues)
- [Development Issues](#development-issues)

## Installation Issues

### Issue: `pnpm: command not found`

**Symptoms**:
```bash
$ pnpm install
bash: pnpm: command not found
```

**Solution**:
```bash
# Enable corepack (Node.js 16.13+)
corepack enable

# Or install pnpm globally
npm install -g pnpm

# Verify installation
pnpm --version
```

**Why this happens**: Anvil requires pnpm 10.17.1+ which is not installed by default.

---

### Issue: Build fails with TypeScript errors

**Symptoms**:
```bash
$ pnpm build
Error: Cannot find module '@anvil/core'
```

**Solution**:
```bash
# Clean build artifacts
rm -rf dist/ node_modules/.cache

# Reinstall dependencies
pnpm install

# Build packages in order
pnpm build

# If still failing, build packages individually
npx nx build core
npx nx build adapters
npx nx build cli
```

**Why this happens**: TypeScript project references require packages to be built in dependency order.

---

### Issue: `anvil: command not found` after installation

**Symptoms**:
```bash
$ anvil validate spec.md
bash: anvil: command not found
```

**Solution**:

**Option 1** - Global link:
```bash
cd cli
pnpm link --global

# Verify
which anvil
anvil --version
```

**Option 2** - Use directly:
```bash
cd cli
node dist/index.js validate spec.md
```

**Option 3** - Use npx:
```bash
cd cli
npx tsx src/index.ts validate spec.md
```

**Why this happens**: CLI needs to be globally linked or run directly.

---

## Format Detection Issues

### Issue: Format not detected or low confidence

**Symptoms**:
```bash
$ anvil validate plan.md
✗ Error: Could not detect format (confidence too low: 25%)
```

**Solutions**:

**Solution 1** - Explicitly specify format:
```bash
anvil validate plan.md --format speckit
# or
anvil validate plan.md --format bmad
# or
anvil validate plan.md --format generic
```

**Solution 2** - Improve document structure:

For SpecKit format, add:
```markdown
# Spec: <title>

## Authors
- Name <email>

## Plan
### Phase 1
...
```

For BMAD format, add:
```markdown
---
title: Feature Name
version: 1.0.0
status: draft
---

## Problem Statement
...

## Requirements
...
```

**Solution 3** - Use native APS:
```bash
# Convert to APS first
anvil export plan.md --format generic --to aps --output plan.aps.json

# Then use native APS
anvil validate plan.aps.json --native
```

**Why this happens**: Generic markdown documents may not have enough structure for confident detection.

---

### Issue: Wrong format detected

**Symptoms**:
```bash
$ anvil validate prd.md
✓ Detected format: speckit (55% confidence)
# But you wanted BMAD
```

**Solution**:
```bash
# Override detection
anvil validate prd.md --format bmad
```

**Permanent fix** - Improve document structure:
```markdown
---
title: Product Requirements
version: 1.0.0
---
# This YAML front-matter increases BMAD confidence to 95%+
```

**Why this happens**: Adapters compete; highest confidence wins. Add format-specific markers to increase confidence.

---

### Issue: File extension not recognized

**Symptoms**:
```bash
$ anvil validate plan.txt
✗ Error: Unsupported file extension: .txt
```

**Solution**:
```bash
# Rename to .md
mv plan.txt plan.md
anvil validate plan.md

# Or specify format explicitly
anvil validate plan.txt --format generic
```

**Why this happens**: Anvil looks for `.md`, `.json`, `.yaml` extensions by default.

---

## Validation Errors

### Issue: Hash verification failed

**Symptoms**:
```bash
$ anvil validate spec.md
✗ Hash verification failed
Expected: abc123...
Actual:   def456...
```

**Solutions**:

**Solution 1** - Skip hash validation:
```bash
anvil validate spec.md --no-validate-hash
```

**Solution 2** - Regenerate hash:
```bash
# Export to APS (regenerates hash)
anvil export spec.md --to aps --output spec.aps.json

# Validate APS version
anvil validate spec.aps.json
```

**Why this happens**: Document was manually edited after hash was generated. Hashes ensure plan integrity.

---

### Issue: Schema validation errors

**Symptoms**:
```bash
$ anvil validate plan.md
✗ Validation failed: 3 errors

1. Missing required field: 'intent'
2. Invalid type for 'proposed_changes': expected array
3. Unknown field: 'extra_data'
```

**Solution**:

Check your document structure matches the format:

**For SpecKit**:
```markdown
# Spec: <title>              # → intent

## Plan                      # → proposed_changes
### Phase 1: Component
Description here...
```

**For BMAD**:
```markdown
## Problem Statement         # → intent

## Implementation Plan       # → proposed_changes
### Phase 1: Setup
...
```

**For native APS**:
```json
{
  "schema_version": "0.1.0",
  "intent": "Required field",
  "proposed_changes": [
    {
      "type": "create",
      "path": "src/file.ts",
      "description": "...",
      "rationale": "..."
    }
  ]
}
```

**Why this happens**: Plan doesn't match expected schema structure.

---

### Issue: Empty or minimal plan

**Symptoms**:
```bash
$ anvil validate plan.md
✗ Validation failed: Plan has no proposed changes
```

**Solution**:

Add content to your plan:

```markdown
# Feature: Add Authentication

## Overview
Add user authentication to the app.

## Tasks
- [ ] Create User model
- [ ] Implement login endpoint
- [ ] Add authentication middleware

## Files to Modify
- `src/models/user.ts` - Create new
- `src/routes/auth.ts` - Create new
```

Minimal valid structure needs:
- Title/intent
- At least one change/task
- Some description

**Why this happens**: Validator requires minimum content to consider plan valid.

---

## Gate Failures

### Issue: All gates fail with "command not found"

**Symptoms**:
```bash
$ anvil gate spec.md
✗ lint failed: pnpm: command not found
✗ test failed: pnpm: command not found
```

**Solutions**:

**Solution 1** - Install project dependencies:
```bash
# In your project root
pnpm install

# Then try gate again
anvil gate spec.md
```

**Solution 2** - Customize gate commands in `.anvilrc`:
```json
{
  "checks": {
    "lint": {
      "enabled": true,
      "command": "npm run lint"    // Use npm instead
    },
    "test": {
      "enabled": true,
      "command": "npm test"         // Use npm instead
    }
  }
}
```

**Solution 3** - Skip checks that don't apply:
```bash
anvil gate spec.md --skip-checks lint,test
```

**Why this happens**: Gate checks run commands that expect project dependencies installed.

---

### Issue: Coverage check fails (below threshold)

**Symptoms**:
```bash
$ anvil gate spec.md
✗ coverage FAIL (65/100) - Coverage: 65% (threshold: 80%)
```

**Solutions**:

**Solution 1** - Lower threshold temporarily:
```json
// .anvilrc
{
  "checks": {
    "coverage": {
      "enabled": true,
      "threshold": 65  // Match current coverage
    }
  }
}
```

**Solution 2** - Skip coverage check:
```bash
anvil gate spec.md --skip-checks coverage
```

**Solution 3** - Increase test coverage:
```bash
# Add tests to improve coverage
pnpm test:coverage

# View coverage report
open coverage/index.html
```

**Why this happens**: Code coverage is below configured threshold. Gate enforces quality standards.

---

### Issue: Lint check fails

**Symptoms**:
```bash
$ anvil gate spec.md
✗ lint FAIL - 15 linting errors found
```

**Solutions**:

**Solution 1** - Fix linting errors:
```bash
# Auto-fix
pnpm lint

# Or manually fix issues
```

**Solution 2** - Skip lint check temporarily:
```bash
anvil gate spec.md --skip-checks lint
```

**Solution 3** - Adjust lint configuration:
```javascript
// eslint.config.mjs
export default [
  {
    rules: {
      // Relax specific rules
      'no-console': 'warn',  // Change from 'error'
    }
  }
];
```

**Why this happens**: Code has linting violations that need fixing.

---

### Issue: Test check fails

**Symptoms**:
```bash
$ anvil gate spec.md
✗ test FAIL - 3 tests failed
```

**Solutions**:

**Solution 1** - Fix failing tests:
```bash
# Run tests to see failures
pnpm test

# Fix test failures
# Then verify
pnpm test
```

**Solution 2** - Skip test check:
```bash
anvil gate spec.md --skip-checks test
```

**Why this happens**: Tests are failing. Fix tests before proceeding.

---

### Issue: Secrets detected

**Symptoms**:
```bash
$ anvil gate spec.md
✗ secrets FAIL - Found potential secrets: API_KEY in src/config.ts
```

**Solutions**:

**Solution 1** - Remove secrets from code:
```javascript
// ❌ Bad - hardcoded secret
const API_KEY = "sk-1234567890abcdef";

// ✅ Good - use environment variables
const API_KEY = process.env.API_KEY;
```

**Solution 2** - Use `.env` files:
```bash
# .env
API_KEY=sk-1234567890abcdef

# .gitignore
.env
```

**Solution 3** - Adjust secret patterns:
```json
// .anvilrc
{
  "checks": {
    "secrets": {
      "enabled": true,
      "patterns": ["password", "secret"]  // Remove "api_key" if needed
    }
  }
}
```

**Why this happens**: Secret scanning detected potential credentials in code.

---

### Issue: Gate timeout

**Symptoms**:
```bash
$ anvil gate spec.md
✗ test TIMEOUT - Check exceeded 60000ms timeout
```

**Solutions**:

**Solution 1** - Increase timeout:
```json
// .anvilrc
{
  "checks": {
    "test": {
      "enabled": true,
      "timeout": 120000  // 2 minutes
    }
  }
}
```

**Solution 2** - Optimize slow tests:
```bash
# Profile tests
pnpm test --reporter=verbose

# Identify slow tests and optimize
```

**Why this happens**: Tests take longer than configured timeout.

---

## Export/Conversion Issues

### Issue: Export fails with format error

**Symptoms**:
```bash
$ anvil export spec.md --to yaml
✗ Error: Cannot export to unsupported format: yaml
```

**Solutions**:

**Available formats**:
- `aps` - Native APS JSON
- `json` - Same as `aps`
- `yaml` - APS in YAML format
- `speckit` - SpecKit format
- `bmad` - BMAD format (if adapter supports export)

```bash
# Correct usage
anvil export spec.md --to aps
anvil export spec.md --to yaml
anvil export spec.md --to speckit
```

**Why this happens**: Format name might be misspelled or format doesn't support export.

---

### Issue: Export creates empty or minimal output

**Symptoms**:
```bash
$ anvil export spec.md --to speckit --output ./output/
✓ Exported to SpecKit
# But ./output/spec.md is nearly empty
```

**Solutions**:

**Solution 1** - Verify source is valid:
```bash
# Validate source first
anvil validate spec.md --verbose

# Check if plan has content
cat spec.md
```

**Solution 2** - Check adapter support:
```bash
# Some adapters may not support full round-trip
# Convert to APS first to see what's preserved
anvil export spec.md --to aps --output debug.aps.json
cat debug.aps.json  # Check content
```

**Why this happens**: Source plan may be minimal, or adapter doesn't support all fields.

---

### Issue: Round-trip conversion loses data

**Symptoms**:
```bash
$ anvil export spec.md --to aps --output temp.json
$ anvil export temp.json --to speckit --output ./output/
# ./output/spec.md is missing some content from original
```

**Solutions**:

**Solution 1** - Use native APS for archival:
```bash
# Keep APS as source of truth
anvil export spec.md --to aps --output archive/spec.aps.json

# Generate SpecKit for viewing
anvil export archive/spec.aps.json --to speckit --output ./specs/
```

**Solution 2** - Improve source format:

Add structure that maps to APS:
```markdown
# Spec: Feature Name

## Authors
- John <john@example.com>

## Plan
### Phase 1: Database
**Files to create**:
- `src/models/user.ts`

**Rationale**: Need user storage
```

**Why this happens**: Not all format features map 1:1 to APS. Some information may be format-specific.

---

### Issue: Output directory not created

**Symptoms**:
```bash
$ anvil export spec.md --to speckit --output /nonexistent/dir/
✗ Error: Output directory does not exist
```

**Solution**:
```bash
# Create directory first
mkdir -p /nonexistent/dir/

# Then export
anvil export spec.md --to speckit --output /nonexistent/dir/
```

**Why this happens**: Anvil doesn't create parent directories automatically.

---

## Performance Issues

### Issue: Validation is slow

**Symptoms**:
```bash
$ anvil validate large-spec.md
# Takes 10+ seconds
```

**Solutions**:

**Solution 1** - Use native APS for large files:
```bash
# Convert once
anvil export large-spec.md --to aps --output spec.aps.json

# Validate APS (faster)
anvil validate spec.aps.json --native
```

**Solution 2** - Split large documents:
```markdown
<!-- Instead of one 5000-line file, split into: -->
spec.md           # Overview
spec-phase1.md    # Phase 1 details
spec-phase2.md    # Phase 2 details
```

**Solution 3** - Disable hash validation:
```bash
anvil validate large-spec.md --no-validate-hash
```

**Why this happens**: Large documents take longer to parse and validate.

---

### Issue: Gate checks take too long

**Symptoms**:
```bash
$ anvil gate spec.md
# Waits 5+ minutes
```

**Solutions**:

**Solution 1** - Run only necessary checks:
```bash
# Skip slow checks
anvil gate spec.md --only-checks lint,secrets
```

**Solution 2** - Optimize test suite:
```bash
# Profile tests
pnpm test --reporter=verbose

# Use test filtering
pnpm test --changed  # Only changed tests
```

**Solution 3** - Increase timeouts:
```json
// .anvilrc
{
  "checks": {
    "test": {
      "timeout": 180000  // 3 minutes
    }
  }
}
```

**Why this happens**: Running full test suite can be slow. Consider parallel execution or test optimization.

---

## Development Issues

### Issue: TypeScript "Cannot find module" errors

**Symptoms**:
```bash
$ pnpm test
Error: Cannot find module '@anvil/core'
```

**Solutions**:

**Solution 1** - Build packages first:
```bash
# Build all packages
pnpm build

# Then run tests
pnpm test
```

**Solution 2** - Build specific package:
```bash
npx nx build core
npx nx build adapters
pnpm test
```

**Why this happens**: TypeScript project references require packages to be built before imports work.

---

### Issue: Tests pass locally but fail in CI

**Symptoms**:
```bash
# Local
$ pnpm test
✓ All tests passing

# CI
✗ Tests failed: 5 errors
```

**Solutions**:

**Solution 1** - Match CI Node.js version:
```bash
# Check CI version (.github/workflows/ci.yml)
# Install same version locally
nvm install 20
nvm use 20
pnpm test
```

**Solution 2** - Clean install:
```bash
# Remove all dependencies
rm -rf node_modules/
rm pnpm-lock.yaml

# Fresh install
pnpm install
pnpm build
pnpm test
```

**Solution 3** - Check for environment differences:
```javascript
// Tests might depend on environment variables
// Add .env.test file
NODE_ENV=test
```

**Why this happens**: Differences in Node.js version, dependencies, or environment variables.

---

### Issue: ESM import errors (missing .js extension)

**Symptoms**:
```typescript
// src/utils/helper.ts
import { foo } from './bar';
// ✗ Error: Cannot find module './bar'
```

**Solution**:
```typescript
// Add .js extension (even for .ts files)
import { foo } from './bar.js';
```

**Why this happens**: Anvil uses ESM with `"module": "nodenext"` which requires explicit `.js` extensions.

---

### Issue: Vitest config errors

**Symptoms**:
```bash
$ pnpm test
Error: Config file is outside rootDir
```

**Solution**:

Update `tsconfig.spec.json`:
```json
{
  "compilerOptions": {
    "rootDir": "."  // Not "src"
  },
  "include": ["src/**/*.test.ts", "vitest.config.ts"]
}
```

**Why this happens**: Vitest config must be in TypeScript rootDir.

---

## Getting Help

If you're still experiencing issues:

1. **Check Documentation**:
   - [Quick Start](./QUICK_START.md) - Get started quickly
   - [User Guide](./USER_GUIDE.md) - Comprehensive reference
   - [Examples](./EXAMPLES.md) - Real-world use cases

2. **Search Issues**: [GitHub Issues](https://github.com/EddaCraft/anvil-001/issues)

3. **Ask Questions**: [GitHub Discussions](https://github.com/EddaCraft/anvil-001/discussions)

4. **File a Bug**: [New Issue](https://github.com/EddaCraft/anvil-001/issues/new)
   - Include Anvil version (`anvil --version`)
   - Include Node.js version (`node --version`)
   - Include error messages and logs
   - Provide minimal reproduction example

## Common Error Messages

Quick reference for error messages:

| Error Message | Solution |
|---------------|----------|
| `pnpm: command not found` | Install pnpm: `corepack enable` |
| `anvil: command not found` | Link CLI: `cd cli && pnpm link --global` |
| `Cannot find module '@anvil/core'` | Build packages: `pnpm build` |
| `Format detection failed` | Use `--format` flag to specify format |
| `Hash verification failed` | Use `--no-validate-hash` flag |
| `Validation failed: Missing required field` | Check document structure matches format |
| `Gate check failed: command not found` | Install project dependencies or skip check |
| `Coverage below threshold` | Increase coverage or lower threshold |
| `Timeout exceeded` | Increase timeout in `.anvilrc` |
| `Unsupported format` | Check format name (`aps`, `speckit`, `bmad`, `yaml`) |

---

**Version**: 0.0.0 (Pre-release)
**Last Updated**: 2025-11-09
