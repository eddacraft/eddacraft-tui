# Quick Start Guide

Get Anvil catching issues in your code in under 5 minutes.

## What You'll Achieve

By the end of this guide, you'll have Anvil:

1. Installed and configured in your project
2. Catching anti-patterns (like `any` types) in your code
3. Running automatically on file save (optional)

## Prerequisites

- **Node.js**: 20.x or later
- **pnpm**: 10.17.1 or later
- **Git**: Your project should be a git repository

## Step 1: Install Anvil

Anvil is currently in pre-release. Install from source:

```bash
# Clone the repository
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001

# Install and build
pnpm install
pnpm build

# Link CLI globally
pnpm link:cli

# Verify installation
anvil --version
```

## Step 2: Initialise Your Project

Navigate to your project and run the setup wizard:

```bash
cd /path/to/your/project
anvil init
```

The wizard will:

- Detect your environment (TypeScript, ESLint, testing framework)
- Create `.anvilrc` configuration
- Set up the `.anvil/` directory
- Optionally install git hooks

**Non-interactive mode** (uses defaults):

```bash
anvil init --non-interactive
```

## Step 3: Run Your First Check

Check your changed files for issues:

```bash
# Check git-changed files
anvil check --changed

# Or check specific files
anvil check src/api/*.ts
```

**Example output** (when issues are found):

```
Checked 3 changed file(s)

Warnings:

⚠ [AP-003] Explicit any type detected
  src/api/handler.ts:42
  Using 'any' defeats type safety
  Fix: Define a proper type or use 'unknown'

⚠ [AP-004] @ts-ignore directive found
  src/utils/parser.ts:18
  Type error ignored without fixing root cause
  Fix: Address the underlying type error

Summary:
  Total: 2
  Warnings: 2
  Time: 45ms

ℹ Warnings found but none are blocking
```

**When everything is clean**:

```
✓ No warnings found
```

## Step 4: Enable Watch Mode (Optional)

For real-time feedback as you code:

```bash
anvil watch --source
```

This watches your source files and runs checks automatically when you save.

**Output**:

```
ANVIL WATCH

  Mode: source files → check
  Patterns: src/**/*.ts, **/*.tsx
  Git filter: unstaged changes only

  ◉ Watching for changes... (Ctrl+C to stop)

  [14:32:05] Change detected: src/api/handler.ts
  [14:32:05] ✓ 0 warnings (23ms)

  [14:35:12] Change detected: src/utils/parser.ts
  [14:35:12] ⚠ 1 warning (31ms)
             AP-003: Explicit any type detected
```

## Step 5: Set Up Git Hooks (Optional)

Run checks automatically before commits:

```bash
# Install pre-commit hook
anvil hooks install
```

This adds a pre-commit hook that runs `anvil check --changed --staged`.

**Skip hooks when needed**:

```bash
ANVIL_SKIP_HOOKS=1 git commit -m "WIP: work in progress"
```

## What Anvil Catches

### Anti-Patterns

| Pattern                      | Why It Matters                |
| ---------------------------- | ----------------------------- |
| Broad `/* eslint-disable */` | Silences all linting          |
| Explicit `any` type          | Defeats type safety           |
| `@ts-ignore` directive       | Ignores errors without fixing |
| Empty catch blocks           | Silently swallows errors      |

### Architecture Violations

Anvil detects when code crosses architectural boundaries you've defined:

```
⚠ [ARCH-001] New cross-boundary dependency
  src/api/handler.ts → src/database/queries.ts
  API layer should not directly access database layer
```

## Suppressing Warnings

When you intentionally need to bypass a check, use suppression comments:

```typescript
// @anvil-ignore AP-003: Third-party SDK requires any for callback
const handler = sdk.createHandler(callback as any);
```

Suppressions require an explanation and are tracked in reports.

## Common Workflows

### Before Committing

```bash
anvil check --changed --staged
```

### During Development

```bash
anvil watch --source
```

### In CI/CD

```yaml
# .github/workflows/anvil.yml
- uses: ./.github/actions/anvil-check
```

### Quick Health Check

```bash
anvil status   # See configuration and recent results
anvil doctor   # Diagnose setup issues
```

## Configuration

Your `.anvilrc` controls Anvil's behaviour:

```json
{
  "checks": {
    "antipattern": {
      "enabled": true,
      "patterns": ["AP-001", "AP-003", "AP-004", "AP-006"]
    },
    "architecture": {
      "enabled": true
    }
  },
  "watch": {
    "patterns": ["src/**/*.ts", "src/**/*.tsx"],
    "debounceMs": 300
  }
}
```

## Next Steps

- **[User Guide](./USER_GUIDE.md)** — Complete command reference
- **[Examples](./EXAMPLES.md)** — Real-world workflows
- **[Troubleshooting](./TROUBLESHOOTING.md)** — Common issues

## Quick Reference

```bash
# Check changed files
anvil check --changed

# Check staged files (pre-commit)
anvil check --changed --staged

# Watch source files
anvil watch --source

# Verbose output with fix suggestions
anvil check --changed --verbose

# JSON output for CI/CD
anvil check --changed --json

# Project health check
anvil status

# Diagnose setup issues
anvil doctor
anvil doctor --fix
```

## Getting Help

- **Documentation**: [docs/](.)
- **Issues**: [GitHub Issues](https://github.com/EddaCraft/anvil-001/issues)

---

**Ready for more?** See the [User Guide](./USER_GUIDE.md) for comprehensive
documentation.
