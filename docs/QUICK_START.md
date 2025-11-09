# Quick Start Guide

Get started with Anvil in 5 minutes. This guide will walk you through
installation and initialising your first project.

## What is Anvil?

Anvil validates planning documents (SpecKit, BMAD, or any markdown plan) and
runs quality gates (lint, test, coverage, secrets) to ensure changes are safe
before execution. It works with your existing planning formats—no need to change
how you write plans.

## Installation

### Prerequisites

- **Node.js** 20.x or 22.x (minimum: 20.0.0)
- **pnpm** 10.17.1 or higher

### Install Anvil (Pre-release)

> **Note**: Anvil is currently in pre-release. Once published to npm, you'll be
> able to install with: `npm install -g @anvil/cli`

**Current installation method**:

```bash
# Clone the repository
git clone https://github.com/EddaCraft/anvil-001.git
cd anvil-001

# Install dependencies
pnpm install

# Build all packages and link CLI globally
pnpm link:cli
```

### Verify Installation

```bash
# Check that anvil is available
anvil --version

# You should see version information
```

## Initialise Your Project

Run `anvil init` in your project directory to set up Anvil:

```bash
cd /path/to/your/project
anvil init
```

This will:

- Detect your development environment (ESLint, Vitest/Jest, TypeScript, etc.)
- Create `.anvilrc` configuration with recommended settings
- Set up directory structure (`.anvil/`, `docs/plans/`)
- Optionally create example planning documents
- Update `.gitignore` with Anvil patterns

**Interactive setup**:

```
🔨 Initialising Anvil in current project...

Detected environment:
  Project: my-app
  Package Manager: pnpm
  Git: ✓
  TypeScript: ✓
  ESLint: ✓
  Testing: Vitest

? Where should planning documents be stored? (docs/plans)
? Which planning format do you use? (Use arrow keys)
❯ SpecKit (GitHub spec-kit format)
  BMAD (PRD/Architecture format)
  Generic Markdown
  Skip example generation
? Create example planning document? (Y/n)
? Configuration template: (Use arrow keys)
❯ Basic (80% thresholds, recommended)
  Strict (90% thresholds, production-ready)
  CI-optimised (minimal checks, fast)
? Enable ESLint gate? (Y/n) (detected)
? Enable test gate? (Y/n) (detected)
? Enable coverage gate? (Y/n)
? Coverage threshold (0-100): (80)
? Enable secret scanning? (Y/n)
```

**Non-interactive mode**:

```bash
# Use defaults
anvil init --non-interactive
```

## Your First Validation

After running `anvil init`, you'll have example planning documents ready to
validate.

### Option 1: Validate Generated Example

If you created an example during init:

```bash
# Validate the example plan
anvil validate docs/plans/example-spec.md

# You'll see output like:
# ✓ Detected format: speckit (95% confidence)
# ✓ Plan is valid
# ✓ All validation checks passed
```

### Option 2: Use an Existing Plan

If you have a SpecKit, BMAD, or any markdown planning document:

```bash
# Validate your plan
anvil validate path/to/your/spec.md
```

### Option 3: Create a Sample Plan

Create a simple `plan.md` file:

```bash
cat > plan.md << 'EOF'
# Feature: Add User Authentication

## Overview
Add secure user authentication to the application.

## Requirements
- Implement login/logout functionality
- Add password hashing with bcrypt
- Create user session management

## Tasks
- [ ] Create User model with email and password fields
- [ ] Implement login endpoint
- [ ] Add logout endpoint
- [ ] Write authentication middleware
- [ ] Add tests for auth flow

## Acceptance Criteria
- Users can register with email and password
- Passwords are hashed and stored securely
- Users can log in and receive a session token
- Session tokens expire after 24 hours
- All authentication endpoints have tests
EOF
```

Now validate it:

```bash
anvil validate plan.md
```

## Run Quality Gates

Quality gates ensure your changes meet quality standards:

```bash
# Run all quality checks
anvil gate plan.md

# You'll see a table of check results:
# ┌──────────┬────────┬─────────┬─────────────────────────────┐
# │ Check    │ Status │ Score   │ Message                     │
# ├──────────┼────────┼─────────┼─────────────────────────────┤
# │ lint     │ ✓ PASS │ 100/100 │ No linting errors found     │
# │ test     │ ✓ PASS │ 100/100 │ All tests passing           │
# │ coverage │ ✓ PASS │  85/100 │ Coverage: 85% (≥80%)        │
# │ secrets  │ ✓ PASS │ 100/100 │ No secrets detected         │
# └──────────┴────────┴─────────┴─────────────────────────────┘
```

**Note:** Gate checks run against your repository code, not the plan itself. If
you don't have tests or linting set up, some checks may be skipped.

## Export Between Formats

Convert your plan to different formats:

```bash
# Convert to APS (Anvil's internal format)
anvil export plan.md --to aps --output plan.aps.json

# Convert to YAML
anvil export plan.md --to yaml --output plan.yaml

# Convert to SpecKit format
anvil export plan.aps.json --to speckit --output ./speckit-docs/
```

## Common Workflows

### Workflow 1: Quick Validation

```bash
# Just validate structure and intent
anvil validate spec.md
```

### Workflow 2: Full Quality Check

```bash
# Validate + run all quality gates
anvil validate spec.md && anvil gate spec.md
```

### Workflow 3: CI/CD Integration

Add to your `.github/workflows/ci.yml`:

```yaml
- name: Validate Planning Documents
  run: |
    anvil validate docs/spec.md
    anvil gate docs/spec.md
```

## Supported Formats

Anvil automatically detects these formats:

| Format         | Common Files                     | Detection |
| -------------- | -------------------------------- | --------- |
| **SpecKit**    | `spec.md`, `plan.md`, `tasks.md` | 90-100%   |
| **BMAD**       | `prd.md`, `architecture.md`      | 95-100%   |
| **Generic MD** | `README.md`, `TODO.md`, etc.     | 30-45%    |
| **APS**        | `*.aps.json`, `*.aps.yaml`       | 100%      |

## Next Steps

Now that you've validated your first plan, you can:

1. **Configure Gate Checks** - Customise which quality checks run
   - See [USER_GUIDE.md](./USER_GUIDE.md#configuration) for details

2. **Explore Examples** - Learn common workflows
   - See [EXAMPLES.md](./EXAMPLES.md) for real-world use cases

3. **Troubleshoot Issues** - Fix common problems
   - See [TROUBLESHOOTING.md](./TROUBLESHOOTING.md) for solutions

4. **Read CLI Reference** - Learn all available commands
   - See [cli/README.md](../cli/README.md) for complete reference

## Quick Reference

```bash
# Validate a plan
anvil validate <plan-file>

# Run quality gates
anvil gate <plan-file>

# Export to another format
anvil export <plan-file> --to <format>

# Get help
anvil --help
anvil validate --help
anvil gate --help
```

## Getting Help

- **Documentation**: [docs/](.)
- **CLI Reference**: [cli/README.md](../cli/README.md)
- **Issues**: [GitHub Issues](https://github.com/EddaCraft/anvil-001/issues)

## What's Next?

Anvil is under active development. Current capabilities:

- ✅ **Validate** planning documents in multiple formats
- ✅ **Quality Gates** (lint, test, coverage, secrets)
- ✅ **Format Conversion** between SpecKit, BMAD, APS

Coming soon:

- ⏳ **Apply** changes with snapshot-based rollback
- ⏳ **Policy Engine** (OPA/Rego) for custom governance
- ⏳ **GitHub Action** for automatic PR validation

---

**Ready to dive deeper?** Check out the [User Guide](./USER_GUIDE.md) for
comprehensive documentation.
