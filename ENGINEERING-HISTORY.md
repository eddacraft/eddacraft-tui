# Engineering History

This document records the complete technical history of changes for each
release.

It includes internal improvements, refactors, infrastructure changes, and
development work that are not included in the public
[product changelog](./CHANGELOG.md).

This log describes the intent and scope of engineering work. It does not
document implementation specifics, internal infrastructure topology, or security
control details. Security changes are recorded at a level that acknowledges the
work without disclosing exploit mechanics.

## [Unreleased]

Product changelog: Rust CLI, beta auth, Ratatui TUI, Rust kernel, json-render
package, kernel benchmarks, TypeScript 6.0 migration.

### Features

- **Rust CLI rewrite** — full port of all 16 CLI subcommands from Node.js
  Commander.js to Rust clap, with JSON output mode, structured error handling,
  and stdin TTY detection (`RCLI-005..034`)
- **Rust kernel engine** — file watcher with debounce/backpressure, tree-sitter
  parser with AST cache, petgraph symbol graph, trust annotation, incremental
  graph updates, architecture config loader, invariant evaluation (cross-layer,
  new-dep, public API, privilege), event emitter, watch mode, embedded mode,
  dual-run harness, engine mode selection (`KERN-010..044`)
- **Ratatui TUI surfaces** — all Ink surfaces ported to native Ratatui:
  welcome, status, doctor, audit, init, browser, gate, watch, tutorial
  (`PORT-010..044`). Shared component library extracted into `eddacraft-tui`
  crate (`RATS-001..007`)
- **Rust engine checks** — secret detection, anti-pattern detection, command
  safety validation ported to Rust with architecture check parity tests and
  engine mode flag (`RENG-001..006`)
- **Beta authentication** — device-code and OTP passwordless auth, session
  refresh with theft detection, admin approval workflow, invite emails, expired
  code cleanup cron, CLI auth commands, website confirmation page (`BAUTH-001..020`)
- **Kernel benchmarks** — criterion micro-benchmarks for watcher, parser,
  graph-query, policy-scaling, and cold-start paths (`BENCH-001..006`)
- **@eddacraft/json-render** — JSON-driven dashboard rendering package with
  schema validation and error boundary (`json-render`)
- **Rust policy crate** — `anvil-policy` scaffold with PolicyLoader and
  OpaExecutor, wired to CLI policy and architecture commands
- **TUI branding** — brand block logo, watermark in shell chrome footer,
  surface-specific help text, Esc/back navigation from all surfaces
- **CLI filters** — `--confidence` and `--since` for `edda list`, login aliases
  for auth commands

### Bug Fixes

- Terminal restore after subprocess failures aborts hub loop cleanly
- Docs errors shown inline instead of flashing to console
- Empty `.anvilrc` rejected in doctor check
- Clap parse failures handled in JSON error path
- Windows path separators normalised in export tests
- Relative imports resolved correctly in kernel graph
- Side-effect module imports handled in parser
- `PrivilegeExpansion` suppressed for already-privileged symbols
- Baseline policy evaluation runs before first watch snapshot
- External trust level preserved in `annotate_trust`
- Watch collections capped to prevent unbounded memory growth
- Pass rate calculation corrected (no double-counting)
- File walker prunes ignored directories during traversal
- Graph uses deterministic import resolution ordering
- Kernel excludes ignored directories from OS-level file watches
- TUI welcome layout adapts to 24-row terminals with centred layout
- TUI position indicator only shown when Issues panel is focused
- TUI audit list viewport scrolling fixed
- TUI watch flicker reduced with dirty-flag rendering
- Loading frame shown during surface transitions
- Select widget highlight extends across full row
- API returns 500 on refresh signing errors instead of `valid:false`
- Licence signing guards against NaN TTL values
- API rejects unparsable `tokenExpiresAt` instead of falling back to max TTL
- API catches `signLicence` errors in auth handlers
- Redundant `.catch()` removed; explicit Buffer encoding added
- `init` surface `should_quit` includes confirmed state

### Refactoring

- Shared TUI components extracted into `eddacraft-tui` crate (select widget,
  theme, keyboard handling)
- `run_subprocess` helper extracted for gate/watch menu options
- Surface trait methods used in `watch_loop` exit check
- Dynamic directory registration and `WatcherHandle` pattern in kernel

### Dependencies

- TypeScript upgraded to 6.0 across all workspace packages (`MAINT-011`) with
  `ignoreDeprecations` for backward compatibility
- Node engine floor raised to >= 22 (>= 22.13.0 in install script)
- Rust toolchain bumped from 1.88.0 to 1.94.0
- fast-xml-parser >= 5.5.6 (`CVE-2026-33036`)
- picomatch and smol-toml overrides for CVE fixes
- undici and yauzl security patches
- flatted and socket.io-parser overrides
- rustls-webpki bumped from 0.103.9 to 0.103.10
- 21 dev dependency updates, 11 production dependency updates (Dependabot)
- eslint-react v2 rule renamed to v3 equivalent
- Incompatible eslint-react v3 and vite v8 bumps reverted

### Infrastructure

- **oxlint + oxfmt** — adopted as first-pass linter and formatter, replacing
  prettier
- Windows and macOS cross-compilation in Rust CI (`KERN-044`)
- Cross-compilation skipped on feature branch PRs
- macOS runner updated to `macos-latest` for x86_64 targets
- Native `aarch64` cross-compilation replaces `cross` tool
- Security scanning CI: trufflehog pinned to v3.94.1 (v4 does not exist)
- Transactional package build step added before unit tests
- `.claude/` excluded from formatting, linting, and CI triggers

### Testing

- Render snapshot tests for all TUI surfaces
- Unit tests for previously untested CLI commands
- Integration test suite for Rust engine checks
- Kernel type validation tests
- Architecture check parity tests (`RENG-004`)
- Dual-run harness for comparing Rust and legacy engines (`KERN-042`)
- Welcome snapshot tests regenerated with help text updates

### Documentation

- Unified UI context spec covering all customer-facing surfaces
- Dependabot alert remediation skill design spec
- Kernel benchmarking spec
- Rust CLI replacement design spec and APS module
- APS planning overhaul — themed releases, completed index, gap modules
- TUTOR module for interactive tutorial
- TUIDASH module for Ratatui json-render spec interpreter
- AGOV agent governance patterns module
- CPACKS compliance policy packs module (28 work items)
- RCLI Tiers 2 and 3 APS modules
- LAC module and TS-vs-Rust ADR
- Beta auth, observability, and IaC themed releases
- WELCOME module for restore-welcome-screen plan
- Intent ledger governance module
- Anvil product marketing/information sheet
- Pitch deck pipeline with orchestrator agent
- Install script, beta testing guide, and release polish docs
- Release runbook updated with dev-to-main promotion flow
- Docs structure reorganised with cross-reference fixes

### Plans

- BAUTH module marked complete (20/20)
- RATS module marked complete (7/7)
- PORT module marked complete (15/15)
- KERN, PORT, RATS, RENG modules reconciled with implemented code
- MAINT-006 and MAINT-007 marked complete
- Completed wave plans moved to `plans/completed/`

## [0.2.1-beta] — Edda Stack

Product changelog: Edda canonical memories, Ember candidate proposals, Stack
health monitoring.

### Features

- **Edda Stack integration** — contracts, storage, services, CLI commands,
  testing utilities, and documentation for the full Kindling → Ember → Edda
  pipeline (`EDDA-001..019`, `STACK-001..016`)
- **Edda/Ember CLI commands** — `edda list`, `edda show`, `edda promote`,
  `edda retire`, `edda trace`, `ember list`, `ember show`, `ember promote`,
  `stack status`, `stack validate`

## [0.1.3]

### Features

- **Forge pre-commit review pipeline** — pre-commit hook intercepts
  `git commit`, spawns `forge-reviewer` agent for cross-model review via codex
  MCP, runs structured negotiation (max 3 rounds), deferred findings filed as
  GitHub issues (`FORGE`, `FNEG`, `DEFER`, `FTCFG` modules — complete). Internal
  only — not in product changelog.
- **Temper post-push self-healing** — GitHub Actions workflow auto-addresses CI
  review comments, max 2 cycles, triggered by `forge:tempered` label or manual
  dispatch. Internal only — not in product changelog.
- **Rust foundation** — Cargo workspace with shared toolchain configuration,
  `anvil-kernel-types` shared crate, KERN Phase 0 spike crate, `eddacraft-tui`
  shared Ratatui component library. Internal only — not in product changelog.
- **Automated security scanning CI** — static analysis, secret detection,
  dependency audit, and licence compliance pipeline
- **APS always-on awareness skill** — auto-reconciliation agent, auto-generated
  project rules from active modules
- BMAD v6 YAML document type support in adapter with sidecar validation
- APS nested index loading with configurable depth limiting (`CRB-011`)
- APS atomic task locking with `O_EXCL` lock files (`CRB-010`)
- `--json` output flag for `hooks status` and `plan create`
- Tutorial continuation between learning paths with completion persistence
- Interactive release command (`anvil release`)
- Rust CI pipeline with path-based change detection
- ADR-011: Rust core engine architecture decision published

### Bug Fixes

See also: product changelog 0.1.3 Fixed and Improved sections.

- Exit code consistency across CLI commands (`ISS-001`, `ISS-002`, `ISS-005`,
  `ISS-007`)
- Missing Ember database returns clean exit instead of unhandled error
- Hardened query construction in memory store to reject invalid sort parameters
- Comma-separated `--type` values parsed correctly in list commands
- `--no-tui` flag removal from commands that no longer support it
- Spawn await fixes for child process management
- CLI error messages surface network errors instead of raw "fetch failed"
- Dependency audit errors surfaced instead of silently reported as clean
  (`CRB-005`)
- Watch mode signal handler reliability under rapid restarts
- Login `--token` flag deprecated with clear migration guidance
- Policy validate resolves paths from cwd, syncs index count
- Spinner leak on validation error and path validation hardening
- Anchor path validation to workspace root instead of cwd
- Tutorial `--reset core` preserves non-core progress
- Tutorial completion persists when switching topics via picker
- `onComplete` guard fires only on last step
- Authorship output routed to stderr per CLI output stream policy
- Watch mode cross-platform path handling and directory root exclusion
- Chokidar v5 adaptation (no glob support)
- Windows `chmodSync` guarding
- Windows path separator normalisation across test assertions
- `waitUntilExit` return type aligned with Ink's `Promise<unknown>`
- Porcelain parsing handles quoted/renamed paths correctly
- Debug namespace corrected for watch mode
- ESLint config avoids overriding files property when spreading react config
- ESLint config defaults plugin/rule/languageOptions to empty objects
- Command parser restores `sudo -C` flag, excludes assignment tokens,
  deduplicates separator logic
- Command extraction scoping tightened in parser
- Regex evaluation hardened against edge cases
- Template pattern tests iterate `LayersRecord` directly
- Input sanitisation improved for schema parsing
- APS `parseStatus()` accepts prose status aliases
- APS status field uses parseable format instead of heading suffix
- `ResultsDashboard` test assertions made more specific
- Redundant ternary operator simplified in dependency check fix command
- `better-sqlite3` re-added to approved build dependencies
- Transitive dependency vulnerability patched by removing unused intermediary

### Refactoring

- CLI stderr/stdout stream policy standardised — all diagnostic output to
  stderr, structured data to stdout (`CRB-001`). Product changelog: Developer
  section.
- Git hook scripts consolidated to single source of truth (`CRB-002`)
- `process.exit()` calls replaced with thrown `CliError`/`CliExit`
- `historical-analyser` renamed to `historical-analyzer` (`CRB-021`)
- Barrel export removed from CLI, output path validation added
- Monorepo restructure: core split into contracts, ports, core, runtime, policy
  (`MONO-004..008`)
- Platform packages extracted: config, storage, crypto (`MONO-009..011`)
- APS `maxDepth` parameter removed from plan loader (unused)
- Forge shell scripts hardened (`PBLU` wave 2)
- 57 post-beta launch uplift items completed (`PBLU` module — complete)
- 8 security review backlog items resolved (`SECB` module — complete)

### Dependencies

- minimatch >= 9.0.7 (`CVE-2026-27904`)
- minimatch bumped from 9.0.3 to 10.2.4
- axios >= 1.13.5 (`CVE-2026-25639`)
- svgo patched (billion laughs DoS)
- tar >= 7.5.10
- serialize-javascript, ajv patched
- undici >= 7.18.2
- hono bumped to ^4.12.5
- `@github/copilot` bumped from 0.0.421 to 0.0.423
- ESLint v10 migration
- Subprocess execution hardened across the codebase
- Input validation hardening across parsers, adapters, and APS loader

### Infrastructure

- Release workflow defaults to CLI-only publishing; beta tags always CLI-only
- Release runbook with preflight checklist, dry-run, and incident steps
- Path-based CI change detection with auto-labelling and reusable
  `detect-changes` job
- Remote build cache for lint, typecheck, and test jobs
- 8-core runner for Windows cross-platform tests
- PR template added
- Required checks aligned with path-based ignores
- Docs-only PRs get placeholder job for required checks
- Vercel path-based ignore scripts to skip unnecessary deploys
- `claude-code-review` workflow removed (superseded by Forge)
- Dependabot PRs skip code review workflow
- Build step added before E2E Playwright tests
- Standard Windows runner to unblock cross-platform tests
- pnpm lockfile synced with anvil-api hono specifier
- `.claude/worktrees/` and `.tmp` artefacts added to gitignore
- Domains standardised to `eddacraft.ai` across repo
- Package distribution hardened for production

### Testing

- Baseline tests for 13 previously untested CLI commands (`CRB-029`)
- Git command composition tests (`CRB-014`)
- Symlink escape tests for file-storage guard (`CRB-015`)
- MCP server suites included in root vitest config (`CRB-013`, `CRB-016`)
- Edda/Ember service boundary validation tests
- Tutorial continuation flow test coverage with keyboard shortcut assertions
- ANSI-tolerant assertions for TUI output
- `vi.waitFor` replaced with `tick()` for negative assertions
- Default exports added to `node:fs` and `node:child_process` mocks
- `@vitest-environment node` annotation for child_process mocks
- Cross-platform path normalisation in command parsing assertions
- Windows CI stability: `safeCleanup` replaces bare `rmSync`, tick delays
  between step transitions
- API client trailing slash stripping tests
- Waitlist email delivery status tests

### Tooling

- APS always-on skill with reconciliation agent
- APS project rules auto-generated from active modules
- Forge reviewer agent with codex MCP delegation
- Deferred finding filing utility
- Forge orchestration command with negotiation protocol
- Temper GitHub Actions workflow for post-push self-healing

### Documentation

- Release runbook (`docs/guides/release-runbook.md`)
- Architecture evolution, kernel spec, diagram research docs
- Vision docs (aspirational features, constitutional engineering)
- Edda Stack docs reframed around capabilities
- APS modules added: KERN, RENG, RATS, PORT, EERB, FORGE, DEFER, FNEG, FTCFG,
  SECB, DASH (5 modules), OPAE
- Completed module statuses audited and corrected across index
- Rust core engine decision space (ADR-011)
- ADR-011 superseded notices added to Ink vs Ratatui decision docs
- Observability foundation module and core runbooks
- Public docs accuracy and first-time user experience fixes
- CLI flag references corrected, caution banners for planned features
- Test coverage metrics updated
- BMAD v6 review feedback addressed

## [0.1.2-beta] - 2026-02-22

### Bug Fixes

- CLI error handling improvements
- Watch mode signal handler reliability

## [0.1.1] - 2026-02-21

### Bug Fixes

- Published CLI metadata no longer exposes `workspace:*` runtime dependencies
- Release workflow publishes CLI-required workspace packages, skips already
  published versions
- pnpm lockfile synced for anvil-cli devDependencies

### Documentation

- Release workflow notes updated in README

## [0.1.0] - 2026-02-21

Initial pre-release.

### Features

- Core analysis engine with parallel processing and caching
- 7 high-confidence anti-pattern detectors
- Architecture baseline inference and cross-boundary detection
- Git integration (`--changed`, `--staged`, `--since`)
- Watch mode with sub-2s feedback
- GitHub Action for PR checks
- VS Code extension (anti-pattern detection, gate display, policy violations,
  syntax highlighting, diagnostics)
- OPA/Rego policy framework with remote bundles, checksum and signature
  verification
- 8 architecture templates
- Drift snapshots, comparison, and reporting
- llms.txt export for AI tools
- MCP server with HTTP transport, resources, and prompt templates
- HTML/CSS anti-pattern detection
- Ink-based TUI: init wizard, status dashboard, doctor diagnostics
- Tutorial with scan-watch-fix flow and intelligent first-run experience
- Interactive architecture wizard with live diagram previews
- Multi-agent concurrency coordination system
- Provenance recording with Git AI Standard v3.0.0
- Kindling integration as provenance storage backend
- Coaching nudges system for anti-pattern detection
- Beta access token system with invite/revoke
- Interactive release command
- APS Markdown adapter, APS parsing and validation

### Dependencies

- tar >= 7.5.4
- lodash >= 4.17.23
- undici >= 7.18.2
- diff >= 4.0.4

### Infrastructure

- Monorepo migration: core split into contracts, ports, core, runtime, policy
- Platform packages: config, storage, crypto
- Nx generators and codemods for scaffolding
- esbuild bundling for self-contained npm package
- Pulumi infrastructure (Vercel, Azure DNS, CI/CD)
- Documentation site (Docusaurus)

### Testing

- TUI testing framework (tuistory) with Ink test renderer
- Test quality enforcement system
- Cross-platform test coverage
- ESLint plugin comprehensive test coverage

### Security

- 3 critical and 10 high-severity findings resolved across MCP server, runtime,
  adapters, storage, APS, and VS Code extension
- 4 medium CLI findings resolved
- Binary integrity verification for external tooling
- Credential storage hardened with restrictive permissions
- API response validation strengthened
- Subprocess execution hardened across the codebase

[Unreleased]: https://github.com/EddaCraft/anvil-001/compare/v0.2.1-beta...HEAD
[0.2.1-beta]:
  https://github.com/EddaCraft/anvil-001/compare/v0.1.3...v0.2.1-beta
[0.1.3]: https://github.com/EddaCraft/anvil-001/compare/v0.1.2-beta...v0.1.3
[0.1.2-beta]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.2-beta
[0.1.1]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.1
[0.1.0]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.0
