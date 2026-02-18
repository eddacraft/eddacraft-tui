# Anvil CLI

Command-line interface for Anvil - deterministic development automation
platform.

## Overview

The Anvil CLI provides commands for validating, executing quality gates, and
converting between planning formats (SpecKit, BMAD, APS). It supports automatic
format detection and works seamlessly with your existing planning documents.

## Installation

```bash
npm install -g @eddacraft/anvil-cli
```

### For Contributors

```bash
# From anvil root directory
pnpm link:cli

# To unlink later
pnpm unlink:cli
```

## Available Commands

### `anvil doctor`

Run diagnostic checks and fix common issues.

**Usage:**

```bash
anvil doctor [options]
```

**Options:**

- `--fix` - Auto-fix all fixable issues
- `--json` - Output results as JSON (for CI/CD)
- `--verbose` - Show detailed diagnostic information
- `--tui` - Force TUI mode (interactive display with spinners)
- `--no-tui` - Force plain text mode (for non-TTY environments)

**What it checks:**

1. **System Requirements** - Node.js version (>= 20), git availability
2. **Configuration** - `.anvilrc` exists and is valid JSON/YAML
3. **Hooks** - Git hooks installed and executable
4. **Permissions** - `.anvil/` writable, plans directory readable

**Examples:**

```bash
# Run all diagnostic checks
anvil doctor

# Auto-fix common issues
anvil doctor --fix

# JSON output for CI/CD
anvil doctor --json

# Verbose output
anvil doctor --verbose

# Force plain text (non-TUI) mode
anvil doctor --no-tui
```

**Output:**

```
[ok] Node.js Version: Node.js v22.0.0
[ok] Git Available: git 2.43.0
[ok] Git Repository: Git repository detected
[ok] Configuration File: .anvilrc found
[ok] Configuration Valid: Valid JSON configuration
[ok] Anvil Directory: .anvil/ directory exists
[ok] Husky Installed: Husky directory found
[!] Pre-commit Hook: pre-commit hook not executable
   -> Run: chmod +x .husky/pre-commit
[ok] Anvil Directory Writable: .anvil/ is writable
[ok] Plans Directory Readable: plans/ is readable

All checks passed
9 passed • 1 warnings

1 issue(s) can be auto-fixed with: anvil doctor --fix
```

**Exit Codes:**

- `0` - All checks passed (warnings allowed)
- `1` - One or more checks failed

---

### `anvil init`

Initialise Anvil in the current project.

**Usage:**

```bash
anvil init [options]
```

**Options:**

- `--force` - Overwrite existing .anvilrc if present
- `--non-interactive` - Skip interactive prompts and use defaults

**What it does:**

1. Detects your development environment (ESLint, Vitest/Jest, TypeScript, etc.)
2. Creates `.anvilrc` configuration with recommended settings
3. Sets up directory structure (`.anvil/`, configurable planning directory)
4. Optionally creates example planning documents
5. Updates `.gitignore` with Anvil patterns

**Examples:**

```bash
# Interactive setup (recommended)
anvil init

# Non-interactive with defaults
anvil init --non-interactive

# Force overwrite existing config
anvil init --force
```

**Output:**

```
🔨 Initialising Anvil in current project...

Detected environment:
  Project: my-app
  Package Manager: pnpm
  Git: ✓
  TypeScript: ✓
  ESLint: ✓
  Testing: Vitest

? Where should planning documents be stored? docs/plans
? Which planning format do you use? SpecKit (GitHub spec-kit format)
? Create example planning document? Yes
...

✓ Anvil initialised successfully!

Created files:
  ✓ .anvilrc
  ✓ .anvil/
  ✓ docs/plans/
  ✓ .gitignore (updated)

Example files:
  ✓ docs/plans/example-spec.md
  ✓ docs/plans/example-plan.md
  ✓ docs/plans/example-tasks.md

Next steps:
  1. Review configuration:
     anvil gate:config --list
  2. Validate example plan:
     anvil validate docs/plans/example-spec.md
  3. Run quality gates:
     anvil gate docs/plans/example-spec.md
```

---

## Available Commands (continued)

### `anvil validate <plan>`

Validate a plan in any supported format (APS, SpecKit, BMAD).

**Usage:**

```bash
anvil validate <plan> [options]
```

**Arguments:**

- `<plan>` - Plan file path or plan ID (e.g., `spec.md`, `plan.json`,
  `aps-a1b2c3d4`)

**Options:**

- `-v, --verbose` - Show detailed validation results
- `--format <format>` - Explicitly specify input format (bypasses
  auto-detection)
- `--native` - Skip format detection and treat as native APS
- `--validate-hash` - Validate hash integrity (default: true)

**Examples:**

```bash
# Validate a SpecKit document (auto-detected)
anvil validate spec.md

# Validate with verbose output
anvil validate plan.md --verbose

# Validate native APS plan
anvil validate plan.json --native

# Validate by plan ID
anvil validate aps-a1b2c3d4

# Skip hash validation
anvil validate spec.md --no-validate-hash
```

**Output:**

```
✓ Detected format: speckit (95% confidence)
✓ Plan is valid

Plan Details:
  Source Format:  speckit
  Adapter:        speckit-import-v2
  ID:             aps-a1b2c3d4
  Schema:         0.1.0
  Hash:           e3b0c44298fc1c14...
  Intent:         Add authentication module
  Changes:        7
  Evidence:       0
  Created By:     user@example.com
  Created At:     2025-10-23T12:00:00Z

✓ All validation checks passed
```

---

### `anvil gate <plan>`

Run quality gates on a plan (lint, test, coverage, secrets, policies).

**Usage:**

```bash
anvil gate <plan> [options]
```

**Arguments:**

- `<plan>` - Plan ID or file path

**Options:**

- `-c, --config <path>` - Custom config file path
- `-v, --verbose` - Verbose output
- `--format <format>` - Explicitly specify input format
- `--native` - Skip format detection and treat as native APS
- `--inject` - Inject evidence back into source document (future feature)
- `--skip-checks <checks>` - Comma-separated list of checks to skip
- `--only-checks <checks>` - Only run specified checks (comma-separated)
- `--fail-fast` - Stop on first check failure

**Examples:**

```bash
# Run all gates on a SpecKit document
anvil gate spec.md

# Run specific checks only
anvil gate plan.md --only-checks lint,test

# Skip coverage check
anvil gate spec.md --skip-checks coverage

# Use custom config
anvil gate plan.json --config .anvil/gate-config.json

# Verbose output with native APS
anvil gate plan.json --native --verbose
```

**Output:**

```
⠋ Loading plan...
✓ Plan loaded (format: speckit, 95% confidence)
⠋ Loading gate configuration...
✓ Configuration loaded
⠋ Running quality gates...
✓ Quality gates completed

Gate Results:
┌──────────┬────────┬─────────┬─────────────────────────────┐
│ Check    │ Status │ Score   │ Message                     │
├──────────┼────────┼─────────┼─────────────────────────────┤
│ lint     │ ✓ PASS │ 100/100 │ No linting errors found     │
│ test     │ ✓ PASS │ 100/100 │ All tests passing           │
│ coverage │ ✓ PASS │  85/100 │ Coverage: 85% (threshold: 80%) │
│ secrets  │ ✓ PASS │ 100/100 │ No secrets detected         │
└──────────┴────────┴─────────┴─────────────────────────────┘

Overall: ✓ PASSED (4/4 checks passed)

✓ All quality gates passed!
```

---

### `anvil export <source>`

Export/convert plans between formats (SpecKit ↔ APS ↔ JSON).

**Usage:**

```bash
anvil export <source> --to <format> [options]
```

**Arguments:**

- `<source>` - Source file path

**Required Options:**

- `--to <format>` - Target format: `aps`, `json`, `speckit`

**Options:**

- `--output <path>` - Output file path or directory
- `--from <format>` - Source format (auto-detected if not specified)
- `--compact` - Compact JSON output (no pretty-printing)

**Examples:**

```bash
# Convert SpecKit to APS (JSON)
anvil export spec.md --to aps

# Convert APS to SpecKit format
anvil export plan.json --to speckit --output ./speckit-docs/

# Compact JSON output
anvil export spec.md --to json --compact

# Explicit source format
anvil export plan.md --from speckit --to aps
```

**Output (SpecKit → APS):**

```
⠋ Loading source file...
✓ Loaded from speckit (95% confidence)
⠋ Converting to aps...
✓ Exported to APS
  Output: /path/to/spec.aps.json
  Size:   2847 bytes

✓ Export complete

Next steps:
  - Validate: anvil validate /path/to/spec.aps.json
```

**Output (APS → SpecKit):**

```
⠋ Loading source file...
✓ Loaded from aps (100% confidence)
⠋ Converting to speckit...
✓ Exported to SpecKit format
  Output directory: /path/to/output
  Files created:
    - spec.md
    - plan.md
    - tasks.md

✓ Export complete

Next steps:
  - Validate: anvil validate /path/to/output/spec.md
```

---

### `anvil plan <intent>` (Planned)

Create a new plan from intent description.

**Status:** Deferred to Week 9 (post-BMAD adapter)

**Planned Usage:**

```bash
anvil plan "Add authentication module" --format speckit
```

---

## First-Run Experience

When you run `anvil` for the first time (without any command), a welcome screen
displays explaining Anvil's value proposition and offering quick-start options.

**Detection:**

- Checks for `.anvil/` directory in current or parent directories
- If not found, displays welcome screen
- Creates `.anvil/` marker after display

**Disable Welcome:**

```bash
# Environment variable
ANVIL_SKIP_WELCOME=1 anvil

# Or create .anvil/ directory manually
mkdir .anvil
```

**Welcome Screen:**

```
   ___              _ __
  / _ | ___  _  __ (_) /
 / __ |/ _ \| |/ // / /
/_/ |_/_//_/|___//_/_/

Make AI-generated code changes safe for production.

Anvil validates plans through quality gates, maintains audit trails,
and ensures every change is reversible.

Quick Start:

  anvil init           Set up Anvil in this project
  anvil doctor         Check your environment setup
  anvil --help         See all available commands

This welcome message appears once. Set ANVIL_SKIP_WELCOME=1 to disable.
```

---

## Format Detection

The Anvil CLI automatically detects planning document formats using
content-based analysis (not just file extensions).

**Supported Formats:**

| Format  | Extensions                       | Status      | Confidence |
| ------- | -------------------------------- | ----------- | ---------- |
| SpecKit | `spec.md`, `plan.md`, `tasks.md` | ✅ Complete | 90-100%    |
| BMAD    | `*.md` (PRD/architecture)        | ✅ Complete | 100%       |
| APS     | `*.json`, `*.yaml`               | ✅ Complete | 100%       |

**How it Works:**

1. CLI reads file content
2. Each registered adapter attempts detection
3. Adapters return confidence score (0-100)
4. Highest confidence adapter is selected
5. If confidence < 50%, format is rejected
6. User can override with `--format` flag

**Example Detection Output:**

```
✓ Detected format: speckit (95% confidence)
```

---

## Configuration

### Gate Configuration

Create `.anvilrc` or `gate-config.json` to customize gate behavior:

```json
{
  "checks": {
    "lint": {
      "enabled": true,
      "command": "pnpm lint"
    },
    "test": {
      "enabled": true,
      "command": "pnpm test",
      "timeout": 30000
    },
    "coverage": {
      "enabled": true,
      "threshold": 80
    },
    "secrets": {
      "enabled": true,
      "patterns": ["password", "api_key", "secret"]
    }
  }
}
```

Use custom config:

```bash
anvil gate plan.md --config .anvil/gate-config.json
```

---

## Exit Codes

- `0` - Success (validation passed, gate passed)
- `1` - Failure (validation failed, gate failed, errors)

**Examples:**

```bash
# Use in CI/CD
anvil gate spec.md
if [ $? -eq 0 ]; then
  echo "Quality gates passed!"
else
  echo "Quality gates failed!"
  exit 1
fi
```

---

## Integration with Adapters

The CLI uses the `@eddacraft/anvil-adapters` package for format conversion:

- **FormatDetectionService** - Auto-detects plan formats
- **PlanLoader** - Loads plans in any format and converts to APS
- **AdapterRegistry** - Manages adapter registration and lookup

All commands support multi-format input automatically.

---

## Development

### Project Structure

```
cli/
├── src/
│   ├── commands/          # CLI command implementations
│   │   ├── doctor.ts      # anvil doctor
│   │   ├── export.ts      # anvil export
│   │   ├── gate.ts        # anvil gate
│   │   ├── init.ts        # anvil init
│   │   ├── validate.ts    # anvil validate
│   │   └── welcome.ts     # First-run welcome handler
│   ├── services/          # Core services
│   │   ├── environment-detector.ts  # Environment detection
│   │   ├── first-run-detector.ts    # First-run detection
│   │   ├── format-detection.ts      # Format auto-detection
│   │   └── plan-loader.ts           # Multi-format plan loading
│   ├── tui/               # Terminal UI components (Ink)
│   │   ├── commands/      # TUI command views
│   │   │   ├── doctor/    # Diagnostics TUI
│   │   │   ├── init/      # Init wizard TUI
│   │   │   ├── status/    # Status display TUI
│   │   │   └── welcome/   # Welcome screen TUI
│   │   ├── components/    # Reusable TUI components
│   │   └── utils/         # TUI utilities (theme, TTY detection)
│   ├── types/             # TypeScript type definitions
│   │   ├── command-options.ts   # CLI option types
│   │   ├── command-results.ts   # Result types
│   │   └── services.ts          # Service interfaces
│   └── utils/             # Utilities
│       ├── file-io.ts     # File operations
│       └── output.ts      # Pretty printing
└── README.md
```

### Running Locally

```bash
# Install dependencies
pnpm install

# Build
pnpm build

# Run CLI
node dist/index.js validate spec.md

# Or use tsx for development
npx tsx src/index.ts validate spec.md
```

### Testing

```bash
# Run all tests
pnpm test

# Run with coverage
pnpm test:coverage

# Type checking
pnpm typecheck
```

**Note:** CLI integration tests exist but currently have a vitest configuration
issue (being fixed).

---

## Examples

### Workflow 1: Validate SpecKit Document

```bash
# 1. Create or modify spec.md
vim spec.md

# 2. Validate
anvil validate spec.md --verbose

# 3. Run quality gates
anvil gate spec.md

# 4. If needed, export to APS
anvil export spec.md --to aps --output plan.json
```

### Workflow 2: Convert Between Formats

```bash
# Convert SpecKit to APS
anvil export spec.md --to aps --output plan.json

# Validate APS
anvil validate plan.json

# Convert back to SpecKit
anvil export plan.json --to speckit --output ./speckit-output/
```

### Workflow 3: CI/CD Integration

```yaml
# .github/workflows/anvil.yml
name: Anvil Quality Gates

on: [pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '22'

      - name: Install Anvil
        run: npm install -g @eddacraft/anvil-cli

      - name: Run Quality Gates
        run: anvil gate spec.md
```

---

## Troubleshooting

### Format Detection Issues

**Problem:** Format not detected correctly

**Solution:**

```bash
# Use explicit format flag
anvil validate spec.md --format speckit

# Or skip detection for native APS
anvil validate plan.json --native
```

### Hash Validation Failures

**Problem:** "Hash verification failed"

**Solution:**

```bash
# Skip hash validation if plan was manually edited
anvil validate spec.md --no-validate-hash

# Or regenerate the hash by re-exporting
anvil export spec.md --to aps
```

### Gate Failures

**Problem:** Gate checks failing

**Solution:**

```bash
# See detailed output
anvil gate spec.md --verbose

# Run specific checks only
anvil gate spec.md --only-checks lint

# Skip problematic checks temporarily
anvil gate spec.md --skip-checks coverage
```

---

## Next Steps

- Read [plans/index.aps.md](../../plans/index.aps.md) for project roadmap
- Read [packages/adapters/README.md](../../packages/adapters/README.md) for
  adapter development
- Read [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) for system design

---

## Support

- **Issues:** https://github.com/EddaCraft/anvil-001/issues
- **Documentation:** See project root README.md and docs/

---

**Version:** 0.1.0 (Beta) **Last Updated:** 2026-02-19
