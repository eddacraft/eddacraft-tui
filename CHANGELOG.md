# Changelog

All notable changes to this product are documented here.

This changelog contains customer-relevant changes only. Internal refactors and
engineering maintenance are recorded in the
[Engineering History](./ENGINEERING-HISTORY.md).

## [Unreleased]

## [0.2.0] — Edda Stack

Anvil gains a memory system. Edda Stack introduces three layers — observation,
interpretation, and canonical memory — that let the platform learn from your
codebase over time.

### Added

- **Edda canonical memories** — persistent, version-tracked knowledge store for
  project patterns and decisions (`EDDA`)
  - `anvil edda list` — list memories with filtering by type, confidence, and
    age
  - `anvil edda show <id>` — display full memory details including provenance
    chain
  - `anvil edda promote` — promote an Ember candidate to canonical memory
  - `anvil edda retire` — retire an outdated memory
  - `anvil edda trace` — trace evolution chain and provenance for a memory
- **Ember candidate proposals** — interpretive layer that surfaces patterns from
  observations (`EMBER`)
  - `anvil ember list` — list proposals with filtering by status and type
  - `anvil ember show <id>` — display full proposal details
  - `anvil ember promote` — mark a proposal as promoted
- **Stack health monitoring** — coordination and status for the memory system
  (`STACK`)
  - `anvil stack status` — show Edda Stack health and component status
  - `anvil stack validate` — validate configuration and provenance integrity

## [0.1.3]

Hardening, reliability, and quality-of-life improvements across the CLI.
Prepares the foundation for the Rust core engine (see ADR-011).

### Added

- BMAD v6 YAML document type support in format adapter
- `--json` output flag for `anvil hooks status` and `anvil plan create`
- Tutorial continuation — continue to another learning path after completing one
- APS nested index loading with configurable depth limiting (`CRB-011`)
- APS atomic task locking — prevents race conditions in multi-agent workflows
  (`CRB-010`)

### Improved

- CLI error messages now surface network errors clearly instead of raw "fetch
  failed"
- Dependency audit errors are surfaced instead of silently reported as clean
- Watch mode signal handler is more reliable under rapid restarts
- Tutorial completion persists when switching topics via the picker
- `--reset core` preserves non-core tutorial progress
- Windows compatibility across path handling, permissions, and signal handling

### Fixed

- Exit code consistency across CLI commands (`ISS-001`, `ISS-002`, `ISS-005`,
  `ISS-007`)
- Missing Ember database now returns a clean exit instead of an unhandled error
- Memory store rejects invalid sort parameters instead of producing unexpected
  results
- Comma-separated `--type` values now parsed correctly in list commands

### Security

- Input validation hardening across parsers, adapters, and plan loader
- Subprocess execution hardened across the codebase
- Dependency patches:
  - minimatch >= 9.0.7 (`CVE-2026-27904`)
  - axios >= 1.13.5 (`CVE-2026-25639`)
  - svgo (billion laughs DoS)
  - tar >= 7.5.10
  - serialize-javascript, ajv, undici

### Developer

- CLI stderr/stdout stream policy standardised — all diagnostic output routes to
  stderr, structured data to stdout
- Git hook scripts consolidated to a single source of truth
- Default API URL changed to `eddacraft-api.vercel.app`
- ADR-011: Rust core engine architecture decision published

## [0.1.2-beta] - 2026-02-22

### Fixed

- CLI error handling improvements
- Watch mode signal handler reliability

## [0.1.1] - 2026-02-21

Patch release focused on npm publish/install reliability for
`@eddacraft/anvil-cli`.

### Fixed

- Published CLI metadata no longer exposes `workspace:*` runtime dependencies to
  npm consumers
- Release workflow publishes CLI-required workspace packages and skips already
  published versions

## [0.1.0] - 2026-02-21

Initial pre-release of Anvil — the deterministic development automation platform
that makes AI-generated code safe to merge by catching architecture boundary
violations and anti-patterns at save time.

### Added

- `anvil check <files>` — analyse files for architecture violations and
  anti-patterns
- `anvil check --changed` — git-aware file detection for staged/unstaged changes
- `anvil check --staged` — check only staged files
- `anvil check --since <ref>` — check files changed since a git reference
- `anvil watch --source` — real-time feedback on file changes with sub-2-second
  latency
- 7 high-confidence AI escape-hatch anti-pattern detectors
- Pattern suppression with time-boxing and mandatory explanations
- Architecture boundary detection with automatic baseline inference
- Architecture templates: Layered, Hexagonal, Clean, DDD, Monorepo, Serverless,
  Nx Workspace, Starter
- `anvil architecture visualise` — Mermaid-based dependency graph rendering
- Interactive architecture wizard with live diagram previews
- `anvil drift snapshot` — capture current architecture state
- `anvil drift compare` — show changes between snapshots
- `anvil drift report` — visualise trends over time
- `anvil explain <id>` — deep-dive into warnings with context
- OPA/Rego policy framework with remote bundles, checksum verification, and
  authentication
- `anvil gate` — run quality gates on the codebase
- `anvil init` — visual TUI wizard for project setup
- `anvil status` — quick health check dashboard
- `anvil doctor` — diagnose setup issues
- `anvil tutorial` — interactive scan-watch-fix tutorial
- GitHub Action for PR checks with comment annotations
- VS Code extension with anti-pattern detection, architecture gate display, OPA
  policy violations, and click-to-navigate
- MCP tool server for real-time validation
- llms.txt export for AI tool consumption
- Command safety validation for AI tool commands
- HTML/CSS anti-pattern detection
- APS Markdown adapter for `.aps.md` planning documents
- `anvil plan load`, `anvil plan validate`, `anvil plan status` — APS planning
  document management

### Security

- 17 findings resolved across MCP server, runtime, CLI, adapters, storage, APS,
  and VS Code extension (3 critical, 10 high, 4 medium)
- External binary integrity verification
- Credential storage hardened with restrictive permissions
- API response validation strengthened throughout

[Unreleased]: https://github.com/EddaCraft/anvil-001/compare/v0.1.2-beta...HEAD
[0.2.0]: https://github.com/EddaCraft/anvil-001/compare/v0.1.3...v0.2.0
[0.1.3]: https://github.com/EddaCraft/anvil-001/compare/v0.1.2-beta...v0.1.3
[0.1.2-beta]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.2-beta
[0.1.1]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.1
[0.1.0]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.0
