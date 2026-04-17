# Engineering History

Technical release history for engineers, platform teams, and technical
evaluators.

This log covers architecture, infrastructure, reliability, security, and
delivery changes behind each release. For end-user feature summaries, see the
[Changelog](./CHANGELOG.md).

## [0.3.3-beta]

### Distribution & Release Engineering

- **WinGet distribution pipeline** — Windows release automation now emits and
  submits WinGet manifests for tagged releases, extending the binary
  distribution surface beyond direct install scripts and Homebrew
- **Windows signing groundwork** — Authenticode signing path wired into the
  release pipeline via Azure Trusted Signing and SSL.com integration so Windows
  artefacts can move to signed distribution once identity provisioning clears
- **Release automation hardening** — `scripts/release.sh` tightened around
  preflight validation, bundled test execution, remote state checks, and
  manifest handoff to the release skill
- **Public release promotion** — release automation now flips production GitHub
  releases to `Latest` consistently instead of leaving beta-tagged artefacts
  hidden behind cargo-dist defaults

### CLI & TUI

- **Windows input handling** — Ratatui/crossterm event handling on Windows now
  filters to key-press events only, removing duplicate input in onboarding and
  discovery flows
- **Discovery surface repair** — two-panel layout restored with predictable
  scrolling behaviour and a reliable onboarding reset path
- **Tutorial completion fixes** — tutorial exit code handling, `husky` flow, and
  verify-step sentinel behaviour corrected so scripted onboarding paths complete
  deterministically
- **Installer UX polish** — post-install output now prints a branded next-steps
  block with colour support and direct pointers to `anvil auth login` and
  `anvil welcome`

### API & Operations

- **Admin list endpoints** — read-only waitlist and audit-list surfaces added in
  support of the in-progress admin CLI module (`ADMINCLI-001`–`ADMINCLI-004`)
- **Licence key boot probe** — `anvil-api` now validates the ES256 signing key
  during startup and reports status through `/health`, surfacing secret/config
  failures before auth traffic hits runtime paths
- **Admin approval collision handling** — approval flow now retries
  `user_code` uniqueness collisions and accepts longer codes to reduce
  back-to-back approval failures
- **Structured auth logging** — waitlist and auth routes emit more consistent,
  structured operational logs for support and production debugging
- **DBCON groundwork** — database consolidation module introduced for the Neon
  project merge, including operator-only waitlist pause controls and bridge
  migration work

### CI, Benchmarking & Security

- **Nightly stress benchmarks** — benchmark runner added to CI to catch native
  engine performance regressions outside the tagged release path
- **Dependency remediation** — `follow-redirects` pinned to a non-vulnerable
  range to close a known supply-chain issue
- **ADR coverage** — ADR-024 published for the literate-core agent harness;
  KERN and BENCH modules archived after completion

## [0.3.2-beta]

### CLI Surface

- **Self-update command** — `anvil update` added as an in-place binary updater
  with version detection, asset download, and verification flow (`RCLI`)
- **Admin invite command** — `anvil admin invite` shipped with dual-mode invite
  flow (email plus approval path), extending beta-user operations from the CLI
- **Welcome/onboarding completion** — all WELCOME tasks closed, finishing the
  first-run path with discovery mode, executable tutorial steps, live watch
  demo, fix flow, and hook installation guidance

### Release & Platform

- **Interactive release script** — `scripts/release.sh` now orchestrates
  preflight, branching, tagging, and workflow kickoff, and writes
  `.release/manifest.json` as a handoff contract for the release skill
- **Feature flag operations docs** — feature-flag inventory and governance guides
  published to make ad-hoc flags auditable across runtime surfaces
- **Windows target expansion** — `aarch64-pc-windows-msvc` added to cargo-dist
  configuration, with updater support explicitly deferred pending upstream
  binary availability

### Reliability & Codebase Maintenance

- **OTP query determinism** — `ORDER BY` restored in `findActiveOtpCodes` to
  prevent non-deterministic code selection under concurrent auth traffic
- **SQL centralisation** — inline API-route SQL moved into `db/queries.ts` to
  make data access easier to audit and less error-prone
- **Tutorial/TUI fixes** — tutorial commands brought back in sync with the Rust
  CLI and long audit result lists fixed to scroll correctly
- **Install flow repair** — installer next-step output now prints reliably; the
  Homebrew tap publish path is triggered automatically during release
- **CI stability** — Semgrep version pinned to avoid upstream breakage and OSSF
  Scorecard restricted to the default branch to reduce noisy failures

### Planning & Governance

- **Versioning decision recorded** — ADR-020 published for release/versioning
  policy
- **Decision log introduced** — `DECISION-LOG.md` added as the single-entry
  index for ADR discovery
- **APS maintenance** — completed modules archived and APS workflow rules
  tightened to keep release and planning state aligned
- **Coverage uplift** — 59 unit tests added for previously under-covered
  `anvil-cli` modules (`TCOV`)

## [0.3.1-beta]

### Infrastructure

- **Feature flags module** — shared feature flagging system across TypeScript
  and Rust surfaces (`FLAGS-001`–`FLAGS-009`)
  - Contract schema with JSON Schema validation
  - Runtime resolver with environment-aware flag evaluation
  - Snapshot system for point-in-time flag state capture
  - Telemetry hooks for flag evaluation tracking
  - Exemplar test fixtures
  - Kernel-side feature flag types, resolver, and snapshot mirroring TS surface
  - Feature flag governance, inventory, and reference guides
  - ADR-019: flags–observability alignment decision
- **CI composite actions** — `setup-workspace` action extracted to deduplicate
  Node/pnpm/Nx setup across workflows; `detect-changes` action for path-based
  job filtering
- **CI workflow fixes** — 8 issues resolved: checkout ordering, clippy/rustfmt
  failures, formatting in setup-workspace action
- **Docs-shell app** — Next.js shell application for docs domain proxy with auth
  callback, login, logout routes, JWT/cookie/state libraries, and unit tests
- **Docs upstream scaffolding** — Docusaurus apps for private and public docs
  with middleware, sidebar configs, and Vercel deployment configuration
- **Vercel build skip** — `vercel-ignore-build.sh` script for skipping preview
  deploys on non-release branches

### Documentation & Delivery

- **PR template** — durable link requirement for manual testing; rationale moved
  to section comment
- **README** — lowercase brand usage, Windows aarch64 scope clarification,
  'Anvil Check' action name restored
- **Release doc checklist** — public distribution repo section added; broken
  inline code span fixed; oxfmt formatting applied
- **AGENTS.md** — updated with current conventions

### Dependencies

- pnpm dependency upgrades (vitest, vite, @types/node, globals, @nx/eslint,
  @nx/vite, @vitest/coverage-v8, @github/copilot)
- Cargo dependency upgrades via Cargo.lock refresh
- lint-staged configuration updated

### Tooling

- `aps-cleanup.service` — systemd service for APS status lifecycle automation
- `nx.json` configuration updates

## [0.3.0-beta]

### Platform Foundations

- **Language and runtime baseline** — TypeScript moved to 6.0 across workspace
  packages and the Node engine floor was raised to `>=22`, reducing divergence
  between local and CI environments (`MAINT-011`)
- **Rust toolchain uplift** — toolchain advanced to 1.94.0 with Windows and
  macOS cross-compilation support aligned to the release matrix
- **Linting and formatting refresh** — oxlint adopted as the first-pass linter
  and oxfmt replaced Prettier for the primary formatting path
- **Documentation platform refresh** — Docusaurus upgraded to 3.10 to keep the
  docs stack current with the Rust CLI release

### Performance & Verification

- **Kernel benchmarking in CI** — Criterion benchmarks for critical kernel paths
  and the stress-test harness were wired into CI, with execution scoped to main
  pushes and manual dispatch where appropriate (`BENCH`)
- **Test coverage uplift** — 59 unit tests added for under-covered `anvil-cli`
  modules alongside an integration suite for the checks crate
- **CI modernisation** — GitHub Actions refreshed to current major versions,
  unused jobs removed, and CodeQL added with path scoping to improve signal and
  maintainability

### Architecture & Dependency Governance

- **Dependency refresh** — key build and runtime dependencies updated,
  including Criterion 0.8, Reqwest 0.13, Dirs 6, and Vite 8
- **Architecture decisions published** — ADR-015 (shared packages restructure)
  and ADR-016 (unified config format) recorded the main design decisions behind
  the release

## [0.2.1-beta]

### Platform & Integration

- **Edda/Ember/Stack integration** — contracts and service-layer work matured
  the project-memory foundation introduced in the 0.2.x beta line

### Security & Hardening

- **Parser and adapter hardening** — validation tightened across parsers,
  adapters, and the APS plan loader to reduce malformed-input and edge-case risk
- **Subprocess execution hardening** — command execution paths further locked
  down to reduce shell-safety regressions
- **Dependency remediation** — vulnerable dependencies patched, including
  `minimatch`, `axios`, `svgo`, and `tar`

## [0.1.3]

### CLI & Delivery

- **CLI stream policy** — stdout/stderr behaviour standardised so automation and
  human-readable output are easier to consume consistently
- **Hook script consolidation** — Git hook scripts moved to a single source of
  truth to reduce drift across local and CI execution
- **Default API endpoint update** — default backend URL moved to
  `eddacraft-api.vercel.app`

### Architecture

- **Rust engine decision recorded** — ADR-011 published the architecture
  decision for the Rust core engine direction
