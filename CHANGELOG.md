# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-01-27

Initial pre-release of Anvil - the deterministic development automation platform
that makes AI-generated code safe to merge by catching architecture boundary
violations and anti-patterns at save time.

### Added

#### Core Analysis Engine

- `anvil check <files>` - Analyse files for architecture violations and
  anti-patterns
- `anvil check --changed` - Git-aware file detection for staged/unstaged changes
- `anvil check --staged` - Check only staged files
- `anvil check --since <ref>` - Check files changed since a git reference
- `anvil watch --source` - Real-time feedback on file changes
- Parallel analysis with intelligent caching for sub-2-second feedback
- Architecture boundary detection with automatic baseline inference
- New cross-boundary edge detection

#### Anti-pattern Detection

- 7 high-confidence AI escape-hatch patterns
- Pattern suppression with time-boxing and mandatory explanations
- Configurable severity levels

#### Onboarding Experience

- `anvil init` - Visual TUI wizard for project setup
- `anvil status` - Quick health check dashboard
- `anvil doctor` - Diagnose setup issues
- First-run welcome experience
- Ink-based TUI components

#### CI/CD Integration

- GitHub Action for PR checks
- PR comment annotations
- Status check integration

#### VS Code Extension

- Anti-pattern detection on file save (< 100ms feedback)
- Architecture gate results in tree view
- OPA policy failure display with remediation hints
- Click-to-navigate for all violation types
- APS and Rego syntax highlighting
- Analysis caching for unchanged files
- Diagnostics panel integration

#### Format Adapters

- APS Markdown adapter for native `.aps.md` planning documents
- Extensible adapter registry for format detection
- Support for task-to-change conversion

#### Monorepo Architecture

- Hexagonal architecture with clear separation:
  - `packages/anvil/contracts` - Schemas and types (zero dependencies)
  - `packages/anvil/ports` - Interface definitions
  - `packages/anvil/core` - Pure domain logic
  - `packages/anvil/runtime` - Orchestration and I/O
  - `packages/anvil/policy` - OPA/Rego wrappers
- Platform packages:
  - `packages/platform/config` - Configuration loading
  - `packages/platform/crypto` - Hashing and signing
  - `packages/platform/storage` - File system abstractions
- Tooling packages:
  - `packages/tooling/eslint-config` - Shared ESLint configuration
  - `packages/tooling/tsconfig` - Shared TypeScript configurations
  - `packages/eslint-plugin-anvil` - Custom ESLint rules
- Supporting packages:
  - `packages/adapters` - Format adapters
  - `packages/aps` - APS parsing and validation
  - `packages/edda-stack` - Edda integration
  - `packages/kindling-integration` - Kindling integration

#### Developer Tools

- `tools/generators` - Nx generators for scaffolding
- `tools/codemods` - Import path migration tools
- Documentation site (`apps/docs-site`)

### Security

- Resolved Dependabot alerts via pnpm overrides:
  - tar >= 7.5.4
  - lodash >= 4.17.23
  - undici >= 7.18.2
  - diff >= 4.0.4

### Documentation

- Quick Start Guide
- Complete command reference
- Demo showing Anvil catching real issues
- Actionable error messages

## [Unreleased]

### Planned for v1.1

- `anvil explain <id>` - Deep-dive into warnings
- `anvil drift snapshot` - Capture current state
- `anvil drift compare` - Show changes over time
- `anvil drift report` - Visualise trends
- OPA architecture integration with YAML-first definitions
- Architecture templates (Layered, Hexagonal, Clean, DDD)
- Remote policy bundle support

### Planned for v2.0

- llms.txt export for AI tool integration
- Copilot/Cursor context rules
- MCP tool server

[0.1.0]: https://github.com/EddaCraft/anvil/releases/tag/v0.1.0
[Unreleased]: https://github.com/EddaCraft/anvil/compare/v0.1.0...HEAD
