# Changelog

All notable changes to the Anvil VS Code Extension will be documented in this
file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-12-31

### Added

- **Anti-pattern detection on save** - Real-time warnings for 7 high-confidence
  patterns (AP-001 through AP-007)
- **Architecture gate display** - Tree view showing layer violations, circular
  dependencies, and orphaned modules
- **OPA policy violation display** - Policy failures grouped by policy name with
  remediation hints
- **Click-to-navigate** - All violations link directly to source locations
- **Syntax highlighting** - APS Markdown (`.aps.md`) and Rego (`.rego`) files
- **Analysis caching** - Content-hash based caching to skip re-analysis of
  unchanged files
- **Gate results tree view** - Dedicated panel in Explorer sidebar
- **Status bar indicator** - Shows current validation state
- **CodeLens actions** - Quick access to validate, gate, and export commands
- **Problem panel integration** - Warnings appear in VS Code Problems panel

### Performance

- Fast-path operations (anti-pattern detection) complete in <100ms
- Production bundle size: ~560KB (well under 2MB target)
- Analysis caching with 5-minute TTL and 100-file LRU capacity

### Configuration

- `anvil.autoValidate` - Auto-validate on save (default: true)
- `anvil.validateOnOpen` - Validate when file opens (default: true)
- `anvil.showStatusBar` - Show status bar indicator (default: true)
- `anvil.showCodeLens` - Show inline actions (default: true)
- `anvil.defaultFormat` - Format detection mode (default: auto)
- `anvil.gates.enabled` - Which gates to run
- `anvil.gates.skipInDevelopment` - Gates to skip in dev mode
- `anvil.coverage.threshold` - Minimum coverage percentage
- `anvil.cli.path` - Custom CLI path

### Supported File Types

- `*.plan.md`, `plan.md` - Plan markdown files
- `*.spec.md` - Specification files
- `*.aps.json` - APS JSON format
- `*.aps.md` - APS Markdown format (with syntax highlighting)
- `*.rego` - OPA Rego policy files (with syntax highlighting)
- `*prd*.md` - BMAD PRD documents
- `*architecture*.md` - BMAD architecture documents
