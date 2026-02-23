# Anvil

Deterministic development automation that makes AI-generated code changes safe
for production. Validate plans through quality gates before execution, ensuring
changes meet your team's standards.

## Quick Start

```bash
# Try without installing
npx @eddacraft/anvil-cli tutorial

# Or install globally
npm install -g @eddacraft/anvil-cli
# or: pnpm add -g @eddacraft/anvil-cli
# or: yarn global add @eddacraft/anvil-cli
# or: bun add -g @eddacraft/anvil-cli
anvil tutorial
```

The interactive tutorial takes about 5 minutes and walks you through scanning,
watching, and fixing issues.

## What Anvil Does

- **Quality gates** - validate code changes against architecture rules,
  anti-patterns, and team conventions
- **Plan validation** - check planning documents (APS, SpecKit, BMAD) before
  execution
- **Architecture enforcement** - define boundaries, layers, and dependency rules
- **Real-time watch mode** - validate as you code with instant feedback
- **AI authorship tracking** - trace which changes were AI-generated via Git
  Notes
- **OPA/Rego policies** - write custom rules for your organisation

## Commands

| Command                | Description                     |
| ---------------------- | ------------------------------- |
| `anvil tutorial`       | Interactive guided tutorial     |
| `anvil init`           | Set up Anvil in a project       |
| `anvil check --all`    | Scan codebase for issues        |
| `anvil watch --source` | Real-time validation            |
| `anvil gate`           | Run quality gates               |
| `anvil doctor`         | Diagnostics and troubleshooting |
| `anvil explain <rule>` | Understand a warning            |
| `anvil status`         | Show workspace status           |
| `anvil --help`         | See all commands                |

## Requirements

- Node.js 20.0.0 or later
- A package manager: **pnpm**, **npm**, **yarn**, or **bun**
- Git

## Beta

This is an early beta release. We welcome bug reports and feedback:

- [Report a bug](https://github.com/EddaCraft/anvil-001/issues/new?template=bug_report.md)
- [Request a feature](https://github.com/EddaCraft/anvil-001/issues/new?template=feature_request.md)
- [Share feedback](https://github.com/EddaCraft/anvil-001/issues/new?template=feedback.md)

## Documentation

- [Beta Quickstart](https://eddacraft.ai/beta)
- [CLI Command Reference](https://github.com/EddaCraft/anvil-001/blob/main/apps/anvil-cli/DEVELOPMENT.md)

## Licence

Apache-2.0
