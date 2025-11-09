# Anvil User Guide

Complete guide to using Anvil for validating and executing quality gates on planning documents.

## Table of Contents

- [Introduction](#introduction)
- [Installation](#installation)
- [Supported Formats](#supported-formats)
- [Commands](#commands)
- [Configuration](#configuration)
- [Workflows](#workflows)
- [Best Practices](#best-practices)
- [Integration](#integration)

## Introduction

### What is Anvil?

Anvil is a validation and execution pipeline that makes AI-generated and human-created code changes safe for production. It provides:

- **Format Agnostic Validation** - Works with SpecKit, BMAD, generic markdown, or native APS
- **Quality Gates** - Automated checks for lint, tests, coverage, secrets
- **Format Conversion** - Export between different planning formats
- **Audit Trail** - Track validation history and evidence
- **Safety First** - Designed for production environments

### Key Concepts

**APS (Anvil Plan Specification)**: Anvil's internal hash-stable format. You don't need to use APS directly—Anvil converts your existing formats automatically.

**Adapters**: Plugins that convert between external formats (SpecKit, BMAD) and APS. Format detection happens automatically.

**Quality Gates**: Automated checks that validate code quality before changes are applied. Currently includes lint, test, coverage, and secret scanning.

**Evidence**: Immutable records of validation results and gate checks, providing a complete audit trail.

## Installation

### Prerequisites

- **Node.js**: Version 18.x, 20.x, or 22.x
- **pnpm**: Version 10.17.1 or higher (enforced)
- **Git**: For cloning the repository

### Standard Installation

```bash
# Clone the repository
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001

# Install dependencies
pnpm install

# Build all packages
pnpm build

# Verify installation
pnpm test
```

### CLI Installation

To use Anvil CLI globally:

```bash
# Navigate to CLI package
cd cli

# Build the CLI
pnpm build

# Link globally
pnpm link --global

# Verify
anvil --version
```

### Development Installation

For contributing or developing Anvil:

```bash
# Clone and install
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001
pnpm install

# Build all packages
pnpm build

# Run tests
pnpm test

# Type checking
pnpm typecheck

# Linting
pnpm lint
```

## Supported Formats

Anvil supports multiple planning document formats through its adapter system.

### SpecKit Format

**Description**: GitHub's spec-kit format for software specifications.

**Common Files**: `spec.md`, `plan.md`, `tasks.md`

**Detection Confidence**: 90-100%

**Example Structure**:

```markdown
# Spec: Add User Authentication

## Authors
- John Doe <john@example.com>

## Overview
Implement secure user authentication with JWT tokens.

## Plan

### Phase 1: User Model
Create User model with email and password fields.

**Files to modify**:
- `src/models/user.ts`
- `src/database/schema.sql`

### Phase 2: Authentication Endpoints
Implement login and logout endpoints.

**Files to create**:
- `src/routes/auth.ts`
- `src/middleware/authenticate.ts`

## Tasks
- [ ] Create User model
- [ ] Add password hashing
- [ ] Implement login endpoint
- [ ] Implement logout endpoint
- [ ] Write tests
```

**Usage**:

```bash
anvil validate spec.md
anvil gate spec.md
anvil export spec.md --to aps
```

### BMAD Format

**Description**: PRD and architecture documents following BMAD conventions.

**Common Files**: `prd.md`, `architecture.md`, `requirements.md`

**Detection Confidence**: 95-100%

**Example Structure**:

```markdown
---
title: User Authentication Feature
version: 1.0.0
status: draft
created: 2025-11-09
author: John Doe
---

# User Authentication Feature

## Problem Statement
Users need a secure way to authenticate and access protected resources.

## Solution Overview
Implement JWT-based authentication with email/password login.

## Requirements

### Functional Requirements
- FR-1: Users can register with email and password
- FR-2: Users can log in with valid credentials
- FR-3: Users can log out and invalidate their session
- FR-4: Passwords must be hashed using bcrypt

### Non-Functional Requirements
- NFR-1: Authentication must complete within 200ms
- NFR-2: Password hashing must use bcrypt with cost factor 12
- NFR-3: Sessions expire after 24 hours

## Architecture

### Components
- **User Model**: Stores user credentials
- **Auth Service**: Handles authentication logic
- **JWT Service**: Generates and validates tokens
- **Auth Middleware**: Protects routes

## Implementation Plan

### Phase 1: Database Schema
Create users table with email, password_hash, created_at fields.

**Files to modify**:
- `prisma/schema.prisma`
- `src/database/migrations/`

### Phase 2: Authentication Service
Implement authentication logic.

**Files to create**:
- `src/services/auth.service.ts`
- `src/services/jwt.service.ts`

## Acceptance Criteria
- [ ] Users can register successfully
- [ ] Login returns valid JWT token
- [ ] Protected routes require valid token
- [ ] Invalid credentials are rejected
- [ ] All auth flows have >90% test coverage
```

**Usage**:

```bash
anvil validate prd.md
anvil gate prd.md
anvil export prd.md --to yaml
```

### Generic Markdown Format

**Description**: Any markdown document with planning content.

**Common Files**: `README.md`, `TODO.md`, `plan.md`, `rfc.md`, `adr.md`

**Detection Confidence**: 30-45% (fallback)

**Example Structure**:

```markdown
# Project Roadmap Q4 2025

## Goals
- Launch user authentication
- Improve test coverage to 90%
- Migrate to PostgreSQL

## Q4 Sprint 1: Authentication
**Start**: Nov 1, **End**: Nov 15

Tasks:
- Create user model
- Implement JWT auth
- Add login/logout endpoints
- Write integration tests

## Q4 Sprint 2: Database Migration
**Start**: Nov 16, **End**: Nov 30

Tasks:
- Set up PostgreSQL instance
- Create migration scripts
- Test data migration
- Deploy to staging
```

**Usage**:

```bash
anvil validate TODO.md
anvil export README.md --to aps
```

### Native APS Format

**Description**: Anvil's internal JSON/YAML format.

**Files**: `*.aps.json`, `*.aps.yaml`

**Detection Confidence**: 100%

**Example Structure**:

```json
{
  "schema_version": "0.1.0",
  "plan_id": "aps-a1b2c3d4",
  "intent": "Add user authentication",
  "proposed_changes": [
    {
      "type": "create",
      "path": "src/models/user.ts",
      "description": "Create User model",
      "rationale": "Store user credentials securely"
    },
    {
      "type": "modify",
      "path": "src/routes/index.ts",
      "description": "Add authentication routes",
      "rationale": "Expose login/logout endpoints"
    }
  ],
  "metadata": {
    "created_by": "john@example.com",
    "created_at": "2025-11-09T10:00:00Z"
  }
}
```

**Usage**:

```bash
anvil validate plan.aps.json --native
anvil gate plan.aps.json
```

## Commands

### `anvil validate`

Validate a planning document for schema correctness and integrity.

**Syntax**:

```bash
anvil validate <plan-file> [options]
```

**Options**:

| Option                  | Description                              |
| ----------------------- | ---------------------------------------- |
| `-v, --verbose`         | Show detailed validation output          |
| `--format <format>`     | Override format detection                |
| `--native`              | Treat as native APS (skip detection)     |
| `--validate-hash`       | Validate hash integrity (default: true)  |
| `--no-validate-hash`    | Skip hash validation                     |

**Examples**:

```bash
# Basic validation
anvil validate spec.md

# Verbose output
anvil validate spec.md --verbose

# Skip hash validation
anvil validate spec.md --no-validate-hash

# Force specific format
anvil validate plan.md --format speckit

# Validate native APS
anvil validate plan.json --native
```

**Output**:

```
✓ Detected format: speckit (95% confidence)
✓ Plan is valid

Plan Details:
  Source Format:  speckit
  Adapter:        speckit-import-v2
  ID:             aps-a1b2c3d4
  Schema:         0.1.0
  Hash:           e3b0c44298fc1c14...
  Intent:         Add user authentication
  Changes:        7
  Evidence:       0
  Created By:     john@example.com
  Created At:     2025-11-09T10:00:00Z

✓ All validation checks passed
```

### `anvil gate`

Run quality gates on a planning document.

**Syntax**:

```bash
anvil gate <plan-file> [options]
```

**Options**:

| Option                      | Description                              |
| --------------------------- | ---------------------------------------- |
| `-c, --config <path>`       | Custom gate configuration file           |
| `-v, --verbose`             | Verbose output                           |
| `--format <format>`         | Override format detection                |
| `--native`                  | Treat as native APS                      |
| `--only-checks <checks>`    | Run only specified checks (comma-separated) |
| `--skip-checks <checks>`    | Skip specified checks (comma-separated)  |
| `--fail-fast`               | Stop on first failure                    |

**Examples**:

```bash
# Run all gates
anvil gate spec.md

# Run specific checks only
anvil gate spec.md --only-checks lint,test

# Skip coverage check
anvil gate spec.md --skip-checks coverage

# Use custom configuration
anvil gate spec.md --config .anvil/gate-config.json

# Verbose output
anvil gate spec.md --verbose

# Stop on first failure
anvil gate spec.md --fail-fast
```

**Output**:

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
│ coverage │ ✓ PASS │  85/100 │ Coverage: 85% (≥80%)        │
│ secrets  │ ✓ PASS │ 100/100 │ No secrets detected         │
└──────────┴────────┴─────────┴─────────────────────────────┘

Overall: ✓ PASSED (4/4 checks passed)

✓ All quality gates passed!
```

### `anvil export`

Convert plans between different formats.

**Syntax**:

```bash
anvil export <source-file> --to <format> [options]
```

**Required Options**:

| Option           | Description                              | Values                        |
| ---------------- | ---------------------------------------- | ----------------------------- |
| `--to <format>`  | Target format                            | `aps`, `json`, `yaml`, `speckit`, `bmad` |

**Options**:

| Option                | Description                              |
| --------------------- | ---------------------------------------- |
| `--output <path>`     | Output file or directory path            |
| `--from <format>`     | Source format (auto-detected if omitted) |
| `--compact`           | Compact JSON (no pretty-printing)        |

**Examples**:

```bash
# Convert SpecKit to APS
anvil export spec.md --to aps

# Convert to YAML with custom output
anvil export spec.md --to yaml --output plan.yaml

# Convert APS to SpecKit
anvil export plan.json --to speckit --output ./speckit-docs/

# Compact JSON
anvil export spec.md --to json --compact

# Explicit source format
anvil export plan.md --from speckit --to aps
```

**Output (SpecKit → APS)**:

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

## Configuration

### Gate Configuration

Create `.anvilrc` or specify a custom config file to customize gate behaviour.

**Default Location**: `./.anvilrc`

**Format**: JSON

**Example Configuration**:

```json
{
  "checks": {
    "lint": {
      "enabled": true,
      "command": "pnpm lint",
      "timeout": 30000
    },
    "test": {
      "enabled": true,
      "command": "pnpm test",
      "timeout": 60000
    },
    "coverage": {
      "enabled": true,
      "command": "pnpm test:coverage",
      "threshold": 80,
      "timeout": 60000
    },
    "secrets": {
      "enabled": true,
      "patterns": [
        "password",
        "api_key",
        "secret",
        "token",
        "private_key"
      ]
    }
  }
}
```

**Configuration Options**:

| Option                      | Description                              | Default       |
| --------------------------- | ---------------------------------------- | ------------- |
| `checks.lint.enabled`       | Enable/disable lint check                | `true`        |
| `checks.lint.command`       | Command to run for linting               | `pnpm lint`   |
| `checks.lint.timeout`       | Timeout in milliseconds                  | `30000`       |
| `checks.test.enabled`       | Enable/disable test check                | `true`        |
| `checks.test.command`       | Command to run tests                     | `pnpm test`   |
| `checks.test.timeout`       | Timeout in milliseconds                  | `60000`       |
| `checks.coverage.enabled`   | Enable/disable coverage check            | `true`        |
| `checks.coverage.threshold` | Minimum coverage percentage              | `80`          |
| `checks.secrets.enabled`    | Enable/disable secret scanning           | `true`        |
| `checks.secrets.patterns`   | Patterns to detect as secrets            | See example   |

**Usage**:

```bash
# Use default .anvilrc in current directory
anvil gate spec.md

# Use custom config file
anvil gate spec.md --config .anvil/custom-config.json

# Override with CLI flags
anvil gate spec.md --skip-checks coverage
anvil gate spec.md --only-checks lint,test
```

### Project-Specific Configuration

Create `.anvil/` directory in your project root:

```
my-project/
├── .anvil/
│   ├── config.json           # Main configuration
│   ├── policies/             # OPA policies (future)
│   └── evidence/             # Evidence bundles (future)
├── .anvilrc                  # Quick config
└── spec.md
```

## Workflows

### Workflow 1: Quick Validation

Validate a planning document before committing:

```bash
# 1. Write your plan
vim spec.md

# 2. Validate
anvil validate spec.md

# 3. Commit if valid
git add spec.md
git commit -m "Add authentication spec"
```

### Workflow 2: Full Quality Check

Run complete validation with quality gates:

```bash
# 1. Validate structure
anvil validate spec.md

# 2. Run all quality gates
anvil gate spec.md

# 3. Review results
# If all pass, proceed with implementation
```

### Workflow 3: Format Conversion

Convert between different planning formats:

```bash
# Convert existing SpecKit to APS
anvil export spec.md --to aps --output plan.aps.json

# Validate APS
anvil validate plan.aps.json

# Share APS with team
git add plan.aps.json
git commit -m "Add APS version of spec"

# Convert back to SpecKit if needed
anvil export plan.aps.json --to speckit --output ./specs/
```

### Workflow 4: CI/CD Integration

Add Anvil to your continuous integration pipeline:

**GitHub Actions** (`.github/workflows/anvil.yml`):

```yaml
name: Anvil Quality Gates

on:
  pull_request:
    paths:
      - 'docs/spec.md'
      - 'docs/prd.md'
      - '*.aps.json'

jobs:
  validate-plans:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Node.js
        uses: actions/setup-node@v3
        with:
          node-version: '20'

      - name: Setup pnpm
        uses: pnpm/action-setup@v2
        with:
          version: 10

      - name: Install Anvil
        run: |
          git clone https://github.com/EddaCraft/anvil-001.git
          cd anvil-001
          pnpm install
          pnpm build
          cd cli && pnpm link --global

      - name: Validate Planning Documents
        run: |
          for file in docs/*.md; do
            echo "Validating $file"
            anvil validate "$file" || exit 1
          done

      - name: Run Quality Gates
        run: |
          for file in docs/*.md; do
            echo "Running gates on $file"
            anvil gate "$file" || exit 1
          done
```

**GitLab CI** (`.gitlab-ci.yml`):

```yaml
anvil-validation:
  stage: test
  image: node:20
  script:
    - corepack enable
    - corepack prepare pnpm@latest --activate
    - git clone https://github.com/EddaCraft/anvil-001.git
    - cd anvil-001
    - pnpm install
    - pnpm build
    - cd cli && pnpm link --global
    - cd $CI_PROJECT_DIR
    - anvil validate docs/spec.md
    - anvil gate docs/spec.md
  only:
    changes:
      - docs/*.md
      - "*.aps.json"
```

### Workflow 5: Pre-commit Hook

Validate plans before each commit:

**Create `.husky/pre-commit`**:

```bash
#!/usr/bin/env sh
. "$(dirname -- "$0")/_/husky.sh"

# Get list of changed .md files in docs/
CHANGED_PLANS=$(git diff --cached --name-only --diff-filter=ACM | grep -E '(spec|plan|prd)\.md$')

if [ -n "$CHANGED_PLANS" ]; then
  echo "Validating planning documents..."
  for file in $CHANGED_PLANS; do
    echo "  Checking $file"
    anvil validate "$file" || exit 1
  done
  echo "✓ All planning documents valid"
fi
```

## Best Practices

### Planning Document Structure

**✅ Do:**
- Use clear, descriptive titles
- Include rationale for changes
- List all files to be modified
- Add acceptance criteria
- Version your plans

**❌ Don't:**
- Mix multiple unrelated features in one plan
- Include sensitive data or secrets
- Make plans too vague or ambiguous
- Forget to update plans when requirements change

### Format Selection

**Use SpecKit when:**
- You're working with GitHub repositories
- You want detailed task tracking
- You need spec/plan/tasks separation

**Use BMAD when:**
- You're writing PRDs or architecture docs
- You need comprehensive requirements
- You want front-matter metadata

**Use Generic Markdown when:**
- You have simple planning documents
- You want maximum flexibility
- You're working with existing markdown files

**Use Native APS when:**
- You need programmatic access
- You're building tools on top of Anvil
- You want guaranteed format stability

### Quality Gate Configuration

**Start Conservative**:
```json
{
  "checks": {
    "lint": { "enabled": true },
    "test": { "enabled": true },
    "coverage": { "enabled": true, "threshold": 70 },
    "secrets": { "enabled": true }
  }
}
```

**Gradually Increase Standards**:
```json
{
  "checks": {
    "coverage": { "enabled": true, "threshold": 90 }
  }
}
```

**Project-Specific Overrides**:
- Lower thresholds for legacy projects
- Higher thresholds for critical systems
- Disable checks that don't apply (e.g., coverage for documentation repos)

### Version Control

**Commit Plans with Code**:
```bash
# Good practice
git add spec.md src/
git commit -m "Add authentication: spec + implementation"
```

**Use Branches for Large Changes**:
```bash
git checkout -b feature/authentication
# Work on spec and code
anvil validate spec.md
anvil gate spec.md
git commit -am "Add authentication feature"
```

**Tag Released Plans**:
```bash
git tag -a v1.0-auth -m "Authentication feature release"
git push --tags
```

## Integration

### Node.js/TypeScript Projects

Import Anvil packages directly:

```typescript
import { APSValidator } from '@anvil/core';
import { AdapterRegistry, FormatDetectionService } from '@anvil/adapters';

// Validate a plan
const validator = new APSValidator();
const result = validator.validate(planData);

if (!result.valid) {
  console.error('Validation errors:', result.errors);
}

// Auto-detect format
const detector = new FormatDetectionService();
const format = await detector.detectFormat('./spec.md');
console.log(`Detected: ${format.format} (${format.confidence}% confidence)`);
```

### REST API (Future)

Anvil will provide a REST API for remote validation:

```bash
# Validate via API
curl -X POST https://anvil-api.example.com/validate \
  -H "Content-Type: application/json" \
  -d @plan.aps.json
```

### VS Code Extension (Future)

Real-time validation in your editor:

- Install "Anvil" extension
- Open any planning document
- See validation errors inline
- Run gates with one click

## Next Steps

- **Examples**: See [EXAMPLES.md](./EXAMPLES.md) for detailed use cases
- **Troubleshooting**: See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for common issues
- **CLI Reference**: See [cli/README.md](../cli/README.md) for complete command reference
- **Architecture**: See [ARCHITECTURE.md](./ARCHITECTURE.md) for system design

## Support

- **Documentation**: [docs/](https://github.com/EddaCraft/anvil-001/tree/main/docs)
- **Issues**: [GitHub Issues](https://github.com/EddaCraft/anvil-001/issues)
- **Discussions**: [GitHub Discussions](https://github.com/EddaCraft/anvil-001/discussions)

---

**Version**: 0.0.0 (Pre-release)
**Last Updated**: 2025-11-09
