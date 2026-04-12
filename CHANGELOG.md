# Changelog

All notable changes to this product are documented here.

This changelog contains customer-relevant changes only. Internal refactors and
engineering maintenance are recorded in the
[Engineering History](./ENGINEERING-HISTORY.md).

## [0.3.1-beta] — Docs Cutover & Onboarding Fixes

### Added

- **Docs domain cutover** — `docs.eddacraft.ai` now served via a docs-shell
  proxy with shared-secret middleware protecting upstream apps (`DOCSAUTH2`)
- **Docs landing page** — nordic terminal-themed hub at `docs.eddacraft.ai` with
  navigation to public and gated documentation sections

### Fixed

- **Welcome screen** — first-user onboarding flows restored after regressions in
  0.3.0-beta; council review findings and PR feedback addressed (`WELCOME`)
- **Docs auth** — CI build failures resolved for the domain cutover; upstream
  middleware and proxy hardened from review feedback; Docusaurus `baseUrl`
  deprecation warning suppressed
- **Auth error messages** — raw HTTP errors replaced with user-friendly messages
  in device-code and login flows
- **TUI version display** — shell footer now shows correct version string and
  ViewDocs handler fixed
- **Beta auth e2e** — e2e test harness for authentication flows fixed
- **Build scripts** — `%h` home-directory expansion replaced with absolute path
  to prevent misexpansion under `sudo`
- **Release pipeline** — `aarch64-pc-windows-msvc` removed from cargo-dist
  targets (upstream `axoupdater` lacks ARM64 Windows binaries)

### Improved

- Vercel auth on docs upstream projects replaced with header-based gating for
  simpler deployment
- `docsStateSecret` reused across Pulumi env vars instead of duplicating the
  secret reference
- Vercel preview deploys skip non-release branches via `vercel-ignore-build.sh`

## [0.3.0-beta] — Rust CLI & Native Engine

### Added

- **Rust CLI** — full native rewrite of the CLI in Rust using clap, replacing
  the Node.js/Commander.js implementation (`RCLI`, `RCLI2`)
  - 20 subcommands: check, watch, gate, gate-config, init, wizard, new, status,
    doctor, tutorial, welcome, audit, hooks, export, auth, admin, policy,
    architecture, validate, drift
  - `anvil policy evaluate` and `anvil architecture validate` wired to real OPA
    executor and config loader
  - `anvil auth login` — device-code authentication flow
  - `anvil admin approve` — approve beta access requests
  - `anvil new` — template browser for project scaffolding
  - `anvil wizard` — interactive setup with template scaffolding
  - `anvil audit` — repository scanning for security findings
  - `anvil drift` — architecture drift tracking (snapshot, compare, report,
    list)
  - `anvil validate` — APS plan file validation (structure, format, hashes)
  - `anvil gate-config` — gate check configuration and thresholds
  - `--json` output mode across all commands with structured error reporting
  - `--confidence` and `--since` filters on edda list
  - Node.js CLI archived to `archive/` — single binary, no runtime dependency
- **Beta authentication system** — passwordless device-code and OTP
  authentication for beta users (`BAUTH`)
  - Device code start, confirm, and poll endpoints
  - OTP request and verify endpoints
  - Session refresh with theft detection
  - Admin approve endpoint with invite email
  - Expired code cleanup cron job
  - Auth auto-refresh in CLI
  - Device code confirmation page on website
  - Beta invite and OTP code email templates
  - Resend audience management for waitlist
- **Docs auth gating** — `/anvil` docs gated behind GitHub OAuth via Vercel
  middleware (`DOCSAUTH`)
  - GitHub OAuth callback in BAUTH API
  - Vercel routing middleware with stateless ES256 JWT verification
  - Login, callback, and logout serverless functions
  - Pending approval and error pages for edge cases
  - Pulumi env vars for Key Vault secrets
- **Welcome screen & onboarding** — first-run detection with interactive
  onboarding experience (`WELCOME`)
  - First-run detection service anchored to workspace root
  - Onboarding welcome surface with discovery mode
  - Executable tutorial steps with live file watching demo
  - Fix step with dual-mode editing
  - Hook installer guidance
  - Gate and watch accessible from welcome menu
  - `ANVIL_DEV=1` bypass for local development testing
- **Ratatui TUI surfaces** — native terminal UI replacing Ink/React (`RATS`,
  `PORT`)
  - Welcome screen with brand block logo and watermark
  - Gate and watch accessible from welcome menu
  - Status dashboard, doctor diagnostics, audit results
  - Init wizard, template browser, gate explorer
  - Tutorial orchestrator with policy, architecture, drift, and CI paths
  - Watch dashboard with dirty-flag rendering to reduce flicker
  - Shell chrome with surface-specific help text and footer
  - Esc/back navigation from all surfaces
  - Loading frame during surface transitions
  - Render snapshot tests for all surfaces
- **Rust kernel** — native core engine with file watching, parsing, and graph
  analysis (`KERN`)
  - File watcher with debounce and backpressure
  - Tree-sitter parser with AST cache and symbol extraction
  - Petgraph symbol graph with module-level dependency tracking and cycle
    detection
  - Trust level annotation for graph nodes
  - Incremental graph updates with GraphDelta
  - Architecture config loading from YAML
  - Invariant evaluation framework — cross-layer, new dependency, public API,
    and privilege escalation checks
  - Event emitter with EngineEvent protocol
  - Foreground watch mode with event streaming
  - Embedded mode for one-shot checks
  - Dual-run harness for engine comparison
  - Engine mode flag for Rust/Legacy/Dual selection
  - Rayon parallel scanning for file walks
  - Architecture parity tests validating Rust engine against TypeScript baseline
- **Kernel benchmarks** — criterion micro-benchmarks and stress test harness for
  critical paths (`BENCH`)
  - Watcher saturation, graph memory, incremental throughput, policy scaling,
    and cold start scenarios
  - CI integration on main pushes and manual dispatch
- **@eddacraft/json-render** — JSON-driven dashboard rendering package for
  declarative UI specs with 3 dashboard spec templates
- **Rust engine checks** — native secret detection, anti-pattern detection, and
  command safety validation ported to Rust (`RENG`)
- **Distribution pipeline** — cross-platform binary releases via cargo-dist
  (`DIST`)
  - Binaries for Linux (x86_64, aarch64), macOS (x86_64, aarch64), and Windows
    (x86_64, aarch64)
  - Shell and PowerShell installers served from `install.eddacraft.ai`
  - Homebrew tap (`brew install eddacraft/tap/anvil`)
  - Built-in self-updater (`anvil-update`)
  - Cross-repo release workflow publishing to `EddaCraft/anvil`
- **Scan filter** — test fixture exclusion from check scans (`WELCOME-004`)
- **OPA v1 and Regal linting in CI** — Rego policies migrated to OPA v1 syntax,
  Regal linter added to Rust workflow (`TFIX-003`, `TFIX-004`)
- **Waitlist migration email** — bulk invite existing waitlist users with
  migration email template and personalised sign-off

### Improved

- Shared packages restructured per ADR-015 (flattened `packages/shared/` into
  `packages/platform/`)
- eddacraft-tui extracted to crates.io v0.1.0 for reuse across projects
  (`TUIEXTRACT`)
- Crate namespace renamed to `eddacraft-anvil-*` for crates.io publishing
- TUI welcome screen layout adapts to small terminals (24-row minimum)
- TUI position indicator only shown when Issues panel is focused
- TUI audit list viewport scrolling with inline expansion
- Watch mode excludes ignored directories from OS-level file watches
- Watch collections capped to prevent unbounded memory growth
- Pass rate calculation corrected (no double-counting)
- File walker prunes ignored directories during traversal
- Graph uses deterministic ordering for import resolution
- External trust level preserved correctly in trust annotation
- Architecture baseline output is deterministic (BTreeMap ordering)
- Workspace root computed once per gate run (performance)
- Vercel preview deploys skipped on non-main branches
- Branding updated to lowercase `anvil` and `eddacraft`
- Public docs aligned with Rust CLI: install commands, CI config,
  troubleshooting all updated for native binary (`DOCSYNC`)
- Node.js/npm references removed from all public documentation

### Fixed

- Terminal restore after subprocess failures now aborts hub loop cleanly
- Docs errors shown inline instead of flashing to console
- Empty `.anvilrc` rejected by doctor check
- Clap parse failures handled in JSON error path
- Windows path separators normalised in export tests
- Relative imports resolved correctly in kernel graph
- Side-effect module imports handled properly
- `PrivilegeExpansion` suppressed for already-privileged symbols
- Baseline policy evaluation runs before first watch snapshot
- Atomic file writes use secure permissions at creation, not after
- TOCTOU races removed in directory and file creation
- First-run marker anchored to workspace root (not CWD)
- Import edge line numbers propagated from parser
- Watch coverage file filter leak fixed (no more coverage/ artifacts)
- Watch event adapter double-counting and unbounded queue growth fixed
- Divergent `main`/`dev` branch histories reconciled (`BRECON`)

### Security

- Device-code and OTP authentication hardened with theft detection on session
  refresh
- Docs auth gating prevents unauthenticated access to `/anvil` documentation
- Licence signing guards against NaN TTL values
- API returns 500 on refresh signing errors instead of `valid:false`
- Log inputs sanitised to prevent log injection
- GitHub Action expression injection sanitised in anvil-check action
- All GitHub Actions pinned to commit SHAs
- CI release workflow hardened from council review
- Atomic credential file writes with restrictive permissions
- Dependency patches:
  - fast-xml-parser >= 5.5.6 (`CVE-2026-33036`)
  - @hono/node-server >= 1.19.13 (`CVE-2026-39406`)
  - axios >= 1.15.0
  - picomatch and smol-toml overrides for CVE fixes
  - undici and yauzl security patches
  - flatted and socket.io-parser overrides
  - rustls-webpki bumped to 0.103.10

### Developer

- TypeScript upgraded to 6.0 across all workspace packages (`MAINT-011`)
- Node engine floor raised to >= 22
- Rust toolchain bumped to 1.94.0 with Windows and macOS cross-compilation
- oxlint adopted as first-pass linter, oxfmt replaces prettier
- Criterion benchmarks added for kernel critical paths and wired into CI
- Stress test harness for kernel benchmarking (`BENCH`)
- Test coverage added for watch, doctor, export, auth device flow, status, and
  audit commands (`TCOV`)
- 59 unit tests for under-covered anvil-cli modules
- Integration test suite for checks crate
- GitHub Actions bumped: checkout v6, setup-node v6, download-artifact v8,
  nx-set-shas v5, labeler v6, azure/login v3, pnpm/action-setup v5
- Unused CI jobs removed (Playwright, e2e-harness, tui-tests)
- Benchmarks restricted to main pushes and manual dispatch
- CodeQL workflow added with paths-ignore
- Docusaurus upgraded to 3.10
- Dependency bumps: criterion 0.8, reqwest 0.13, dirs 6, Vite 8
- ADR-015 (shared packages restructure) and ADR-016 (unified config format)
  published

## [0.2.1-beta] — Project Memory & Pattern Detection

See [Engineering History](./ENGINEERING-HISTORY.md) for full technical details.
Edda/Ember/Stack CLI commands shipped in the Node.js CLI; Rust CLI ports are
deferred to a future release (RCLI3).

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

[0.3.1-beta]:
  https://github.com/EddaCraft/anvil-001/compare/v0.3.0-beta...v0.3.1-beta
[0.3.0-beta]:
  https://github.com/EddaCraft/anvil-001/compare/v0.2.1-beta...v0.3.0-beta
[0.2.1-beta]:
  https://github.com/EddaCraft/anvil-001/compare/v0.1.3...v0.2.1-beta
[0.1.3]: https://github.com/EddaCraft/anvil-001/compare/v0.1.2-beta...v0.1.3
[0.1.2-beta]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.2-beta
[0.1.1]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.1
[0.1.0]: https://github.com/EddaCraft/anvil-001/releases/tag/v0.1.0
